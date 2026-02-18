use crate::{
    options::Options,
    shared_types::{PointWithHeight, Source, TileMeta},
};
use core::f64;
use las::{Reader, point::Classification};
use proj::Proj;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rusqlite::{Connection, OpenFlags};
use spade::Point2;
use std::sync::Mutex;
use tilemath::{bbox::BBox, utils::bbox_covered_tiles};

pub fn read(options: &Options, bbox: &BBox, unit_zoom_level: u8) -> Vec<TileMeta> {
    let buffer_m = options.buffer as f64 / options.pixels_per_meter();

    let tile_metas: Vec<_> = bbox_covered_tiles(bbox, unit_zoom_level)
        .map(|tile| TileMeta {
            tile,
            bbox: tile
                .bounds(options.tile_size << (options.zoom_level - unit_zoom_level))
                .to_buffered(buffer_m),
            points: Mutex::new(Vec::<PointWithHeight>::new()),
        })
        .collect();

    let Source::LazIndexDb(path) = options.source() else {
        return tile_metas;
    };

    let bbox_unprojected = options.source_projection.as_ref().map(|source_projection| {
        let bbox_unprojected: BBox = Proj::new_known_crs("EPSG:3857", source_projection, None)
            .expect("Failed to create PROJ transformation")
            .transform_bounds(bbox.min_x, bbox.min_y, bbox.max_x, bbox.max_y, 11)
            .unwrap()
            .into();

        bbox_unprojected
    });

    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();

    let mut stmt = conn.prepare("SELECT file FROM laz_index WHERE max_x >= ?1 AND min_x <= ?3 AND max_y >= ?2 AND min_y <= ?4").unwrap();

    let rows = stmt
        .query_map(
            <[f64; 4]>::from(
                bbox_unprojected
                    .unwrap_or_else(|| BBox::new(bbox.min_x, bbox.min_y, bbox.max_x, bbox.max_y)),
            ),
            |row| row.get::<_, String>(0),
        )
        .unwrap();

    let files: Vec<String> = rows.map(|row| row.unwrap()).collect();

    println!("Reading {} files", files.len());

    files.par_iter().for_each_init(
        || {
            options.source_projection.as_ref().map(|source_projection| {
                Proj::new_known_crs("EPSG:3857", source_projection, None)
                    .expect("Failed to create PROJ transformation")
            })
        },
        |proj, file| {
            println!("READ {file}");

            let mut reader = Reader::from_path(file).unwrap();

            for point in reader.points() {
                let point = point.unwrap();

                if point.classification == Classification::Water {
                    continue;
                }

                if let Some(bbox_unprojected) = bbox_unprojected {
                    if !bbox_unprojected.contains(point.x, point.y) {
                        continue;
                    }
                }

                let (x, y) = proj.as_ref().map_or_else(
                    || (point.x, point.y),
                    |proj| proj.convert((point.x, point.y)).unwrap(),
                );

                if !bbox.contains(x, y) {
                    continue;
                }

                for (i, tile_meta) in tile_metas.iter().enumerate() {
                    if !tile_meta.bbox.contains(x, y) {
                        continue;
                    }

                    tile_metas
                        .get(i)
                        .unwrap()
                        .points
                        .lock()
                        .unwrap()
                        .push(PointWithHeight {
                            position: Point2::new(x, y),
                            height: point.z,
                        });
                }
            }

            println!("DONE {file}");
        },
    );

    tile_metas
}
