use gdal::{Dataset, raster::ResampleAlg};
use ndarray::Array2;
use std::path::Path;
use tilemath::bbox::BBox;

pub fn derived_bbox(path: &Path) -> BBox {
    let dataset = Dataset::open(path).expect("Failed to open GeoTIFF");

    ensure_3857(&dataset, path);

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

pub struct GeoTiffContext {
    dataset: Dataset,
    geotransform: [f64; 6],
    raster_w: isize,
    raster_h: isize,
    no_data: Option<f64>,
}

impl GeoTiffContext {
    pub fn open(path: &Path) -> Self {
        let dataset = Dataset::open(path).expect("Failed to open GeoTIFF");

        ensure_3857(&dataset, path);

        let geotransform = dataset
            .geo_transform()
            .expect("GeoTIFF missing geotransform");

        assert!(
            geotransform[2] == 0.0 && geotransform[4] == 0.0,
            "Rotated/sheared geotransforms are not supported"
        );

        let band = dataset.rasterband(1).unwrap();

        Self {
            raster_w: band.x_size() as isize,
            raster_h: band.y_size() as isize,
            no_data: band.no_data_value(),
            dataset,
            geotransform,
        }
    }

    pub fn read_resampled(&self, bbox: BBox, out_w: usize, out_h: usize) -> Array2<f64> {
        let gt = self.geotransform;

        let px0 = (bbox.min_x - gt[0]) / gt[1];
        let px1 = (bbox.max_x - gt[0]) / gt[1];
        let py0 = (bbox.min_y - gt[3]) / gt[5];
        let py1 = (bbox.max_y - gt[3]) / gt[5];

        let x_min = px0.min(px1).floor() as isize;
        let x_max = px0.max(px1).ceil() as isize;
        let y_min = py0.min(py1).floor() as isize;
        let y_max = py0.max(py1).ceil() as isize;

        let x_off = x_min.clamp(0, self.raster_w);
        let y_off = y_min.clamp(0, self.raster_h);
        let x_end = x_max.clamp(0, self.raster_w);
        let y_end = y_max.clamp(0, self.raster_h);

        if x_end <= x_off || y_end <= y_off {
            return Array2::from_elem((out_h, out_w), f64::NAN);
        }

        let in_w = (x_end - x_off) as usize;
        let in_h = (y_end - y_off) as usize;

        let band = self.dataset.rasterband(1).unwrap();

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

        if let Some(no_data) = self.no_data {
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
        "GeoTIFF CRS is {:?}. This mode currently requires EPSG:3857.\nHint: gdalwarp -t_srs EPSG:3857 {} projected.tif",
        auth_code,
        path.display(),
    );
}
