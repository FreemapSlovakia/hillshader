use clap::{Parser, ValueEnum};
use dem_common::{
    overview::{compose_overview_tile, OverviewResampling as CommonOverviewResampling},
    schema::create_schema,
    tilecodec::{decode_dem_tile, encode_dem_tile, EDGE_BUFFER_PX},
};
use gdal::{raster::ResampleAlg, Dataset};
use lru::LruCache;
use ndarray::{s, Array2};
use rusqlite::Connection;
use std::{
    collections::HashSet,
    fs::{exists, remove_file},
    num::NonZero,
    path::{Path, PathBuf},
};
use tilemath::{bbox::BBox, tile::Tile, utils::bbox_covered_tiles, WEB_MERCATOR_EXTENT};

const INSERT_TILE_SQL: &str =
    "INSERT INTO tiles (zoom_level, tile_column, tile_row, tile_data) VALUES (?1, ?2, ?3, ?4)";

#[derive(Clone, Debug, Parser, PartialEq)]
struct Options {
    /// Output mbtiles file
    output: PathBuf,

    /// Source GeoTIFF file
    #[clap(long)]
    geotiff: PathBuf,

    /// Max zoom level of tiles to generate
    #[clap(long)]
    zoom_level: u8,

    /// Tile size
    #[clap(long, default_value_t = 256)]
    tile_size: u16,

    /// Additional processing buffer in pixels before sampling (cropped before save)
    #[clap(long, default_value_t = 0)]
    buffer: u32,

    /// LRU cache size for overview generation
    #[clap(long, default_value_t = 4096)]
    lru_size: usize,

    /// Resampling method for overview DEM tiles
    #[clap(long, value_enum, default_value_t = OverviewResampling::Lanczos)]
    overview_resampling: OverviewResampling,

    /// Action to take if output file exists
    #[clap(long, value_enum)]
    existing_file_action: Option<ExistingFileAction>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ExistingFileAction {
    Overwrite,
    Continue,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OverviewResampling {
    Lanczos,
    Bilinear,
}

impl From<OverviewResampling> for CommonOverviewResampling {
    fn from(value: OverviewResampling) -> Self {
        match value {
            OverviewResampling::Lanczos => Self::Lanczos,
            OverviewResampling::Bilinear => Self::Bilinear,
        }
    }
}

fn main() {
    let options = Options::parse();

    let r#continue = exists(&options.output).unwrap()
        && match options.existing_file_action {
            Some(ExistingFileAction::Overwrite) => {
                remove_file(&options.output).unwrap();
                false
            }
            Some(ExistingFileAction::Continue) => true,
            None => panic!("Output file already exists. Specify --existing-file-action."),
        };

    let dataset = Dataset::open(&options.geotiff).expect("Failed to open GeoTIFF");

    ensure_3857(&dataset, &options.geotiff);

    let bbox = dataset_bbox(&dataset);

    let conn = Connection::open(&options.output).unwrap();

    if !r#continue {
        create_schema(
            &conn,
            &[
                ("format", "application/x-fm-dem"),
                ("minzoom", "0"),
                ("maxzoom", options.zoom_level.to_string().as_ref()),
            ],
        )
        .unwrap();
    }

    let base_tiles: Vec<_> = bbox_covered_tiles(&bbox, options.zoom_level).collect();

    println!("Rasterizing {} base tiles", base_tiles.len());

    for tile in &base_tiles {
        if r#continue && tile_exists(&conn, *tile) {
            continue;
        }

        let dem = render_tile_dem(&dataset, &options, *tile);

        save_tile(&conn, *tile, dem);
    }

    build_overviews(
        &conn,
        &base_tiles,
        options.tile_size as usize,
        options.lru_size.max(1),
        options.overview_resampling.into(),
    );
}

fn ensure_3857(dataset: &Dataset, path: &Path) {
    let Ok(spatial_ref) = dataset.spatial_ref() else {
        panic!(
            "GeoTIFF has no CRS metadata: {}. Please reproject to EPSG:3857 first.",
            path.display()
        );
    };

    let auth_code = spatial_ref.auth_code().ok();

    assert!(
        auth_code == Some(3857),
        "GeoTIFF CRS is {:?}. This tool currently requires EPSG:3857.\nHint: gdalwarp -t_srs EPSG:3857 {} projected.tif",
        auth_code,
        path.display(),
    );
}

fn tile_exists(conn: &Connection, tile: Tile) -> bool {
    conn.query_row(
        "SELECT 1 FROM tiles WHERE zoom_level = ?1 AND tile_column = ?2 AND tile_row = ?3",
        (tile.zoom, tile.x, tile.reversed_y()),
        |_| Ok(()),
    )
    .is_ok()
}

fn save_tile(conn: &Connection, tile: Tile, dem: Array2<f64>) {
    let buffer = encode_dem_tile(&dem, tile.zoom);

    conn.execute(
        INSERT_TILE_SQL,
        (tile.zoom, tile.x, tile.reversed_y(), buffer),
    )
    .unwrap();
}

fn render_tile_dem(dataset: &Dataset, options: &Options, tile: Tile) -> Array2<f64> {
    let pixel_size_m =
        (2.0 * WEB_MERCATOR_EXTENT) / f64::from((options.tile_size as u32) << options.zoom_level);

    let processing_buffer_px = EDGE_BUFFER_PX + options.buffer as usize;

    let tile_bbox = tile
        .bounds(options.tile_size)
        .to_buffered(processing_buffer_px as f64 * pixel_size_m);

    let out_size = options.tile_size as usize + processing_buffer_px * 2;

    let dem = read_resampled(dataset, tile_bbox, out_size, out_size);
    let crop = options.buffer as usize;
    let expected = tile_size_with_edge(options.tile_size);

    dem.slice(s![crop..(crop + expected), crop..(crop + expected)])
        .to_owned()
}

fn tile_size_with_edge(tile_size: u16) -> usize {
    tile_size as usize + EDGE_BUFFER_PX * 2
}

fn dataset_bbox(dataset: &Dataset) -> BBox {
    let gt = dataset
        .geo_transform()
        .expect("GeoTIFF missing geotransform");

    assert!(
        gt[2] == 0.0 && gt[4] == 0.0,
        "Rotated/sheared geotransforms are not supported"
    );

    let (w, h) = dataset.raster_size();

    let min_x = gt[0];
    let max_y = gt[3];
    let max_x = gt[0] + gt[1] * w as f64;
    let min_y = gt[3] + gt[5] * h as f64;

    BBox::new(
        min_x.min(max_x),
        min_y.min(max_y),
        min_x.max(max_x),
        min_y.max(max_y),
    )
}

fn read_resampled(dataset: &Dataset, bbox: BBox, out_w: usize, out_h: usize) -> Array2<f64> {
    let band = dataset.rasterband(1).unwrap();
    let gt = dataset
        .geo_transform()
        .expect("GeoTIFF missing geotransform");

    assert!(
        gt[2] == 0.0 && gt[4] == 0.0,
        "Rotated/sheared geotransforms are not supported"
    );

    let raster_w = band.x_size() as isize;
    let raster_h = band.y_size() as isize;

    let px0 = (bbox.min_x - gt[0]) / gt[1];
    let px1 = (bbox.max_x - gt[0]) / gt[1];
    let py0 = (bbox.min_y - gt[3]) / gt[5];
    let py1 = (bbox.max_y - gt[3]) / gt[5];

    let x_min = px0.min(px1).floor() as isize;
    let x_max = px0.max(px1).ceil() as isize;
    let y_min = py0.min(py1).floor() as isize;
    let y_max = py0.max(py1).ceil() as isize;

    let x_off = x_min.clamp(0, raster_w);
    let y_off = y_min.clamp(0, raster_h);
    let x_end = x_max.clamp(0, raster_w);
    let y_end = y_max.clamp(0, raster_h);

    if x_end <= x_off || y_end <= y_off {
        return Array2::from_elem((out_h, out_w), f64::NAN);
    }

    let in_w = (x_end - x_off) as usize;
    let in_h = (y_end - y_off) as usize;

    let mut arr = Array2::from_shape_vec(
        (out_h, out_w),
        band.read_as::<f64>(
            (x_off, y_off),
            (in_w, in_h),
            (out_w, out_h),
            Some(ResampleAlg::Bilinear),
        )
        .unwrap()
        .data()
        .to_vec(),
    )
    .unwrap();

    if let Some(no_data) = band.no_data_value() {
        arr.mapv_inplace(|v| {
            if (v - no_data).abs() < 1e-12 {
                f64::NAN
            } else {
                v
            }
        });
    }

    arr
}

fn load_tile(
    conn: &Connection,
    cache: &mut LruCache<Tile, Array2<f64>>,
    tile: Tile,
) -> Option<Array2<f64>> {
    if let Some(dem) = cache.get(&tile) {
        return Some(dem.clone());
    }

    conn.query_row(
        "SELECT tile_data FROM tiles WHERE zoom_level = ?1 AND tile_column = ?2 AND tile_row = ?3",
        (tile.zoom, tile.x, tile.reversed_y()),
        |row| row.get::<_, Vec<u8>>(0),
    )
    .ok()
    .map(|buf| {
        let dem = decode_dem_tile(&buf);
        cache.put(tile, dem.clone());
        dem
    })
}

fn build_overviews(
    conn: &Connection,
    base_tiles: &[Tile],
    tile_size: usize,
    lru_size: usize,
    overview_resampling: CommonOverviewResampling,
) {
    let mut cache = LruCache::<Tile, Array2<f64>>::new(NonZero::new(lru_size).unwrap());
    let mut current: HashSet<Tile> = base_tiles.iter().copied().collect();

    while !current.is_empty() {
        let parents: HashSet<_> = current.iter().filter_map(Tile::parent).collect();

        if parents.is_empty() {
            break;
        }

        println!("Building {} overview tiles", parents.len());

        let mut next = HashSet::new();

        for parent in parents {
            if build_single_overview(conn, &mut cache, parent, tile_size, overview_resampling) {
                next.insert(parent);
            }
        }

        current = next;
    }
}

fn build_single_overview(
    conn: &Connection,
    cache: &mut LruCache<Tile, Array2<f64>>,
    tile: Tile,
    tile_size: usize,
    overview_resampling: CommonOverviewResampling,
) -> bool {
    let children: Vec<_> = tile
        .children_buffered(1)
        .enumerate()
        .filter_map(|(sector, child)| load_tile(conn, cache, child).map(|dem| (sector, dem)))
        .collect();

    let Some(dem) = compose_overview_tile(children, tile_size, f64::NAN, overview_resampling)
    else {
        return false;
    };

    cache.put(tile, dem.clone());
    save_tile(conn, tile, dem);

    true
}
