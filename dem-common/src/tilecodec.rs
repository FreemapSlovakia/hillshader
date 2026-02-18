use ndarray::Array2;
use std::io::Cursor;

pub const EDGE_BUFFER_PX: usize = 2;

pub fn lerc_tolerance_for_zoom(zoom: u8) -> f64 {
    2_f64.powf((20.0 - zoom as f64) / 1.5) / 150.0
}

pub fn encode_dem_tile(dem: &Array2<f64>, zoom: u8) -> Vec<u8> {
    let encoded = lerc::encode(
        dem.mapv(|x| x as f32).as_slice().unwrap(),
        None,
        dem.ncols(),
        dem.nrows(),
        1,
        1,
        0,
        lerc_tolerance_for_zoom(zoom),
    )
    .expect("error encoding lerc");

    zstd::encode_all(Cursor::new(encoded), 0).unwrap()
}

pub fn decode_dem_tile(buffer: &[u8]) -> Array2<f64> {
    let decompressed = zstd::decode_all(Cursor::new(buffer)).unwrap();

    let floats = lerc::decode_auto::<f32>(&decompressed)
        .expect("error decoding lerc")
        .0;

    let len = floats.len();
    let dim = (len as f64).sqrt() as usize;

    assert_eq!(dim * dim, len, "data does not form a square matrix");

    Array2::from_shape_vec((dim, dim), floats.into_iter().map(f64::from).collect()).unwrap()
}
