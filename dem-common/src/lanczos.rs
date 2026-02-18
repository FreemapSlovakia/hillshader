// General Lanczos3 resizing for Array2<f64> with runtime input/output size and cached kernels
use ndarray::{Array2, ArrayView2, ArrayViewMut2, Axis};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::Mutex;
use std::sync::OnceLock;

const LANCZOS_RADIUS: usize = 3;

fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        let pix = PI * x;

        (pix.sin()) / pix
    }
}

fn lanczos3(x: f64) -> f64 {
    if x.abs() < LANCZOS_RADIUS as f64 {
        sinc(x) * sinc(x / LANCZOS_RADIUS as f64)
    } else {
        0.0
    }
}

fn generate_kernel(src_len: usize, dst_len: usize) -> Vec<Vec<(isize, f64)>> {
    let scale = dst_len as f64 / src_len as f64;

    let mut kernel = Vec::with_capacity(dst_len);

    for i in 0..dst_len {
        let center = (i as f64 + 0.5) / scale - 0.5;

        let left = center.floor() as isize - LANCZOS_RADIUS as isize + 1;

        let mut weights = Vec::with_capacity(2 * LANCZOS_RADIUS);

        let mut sum = 0.0;

        for j in 0..2 * LANCZOS_RADIUS {
            let idx = left + j as isize;

            let w = lanczos3(center - idx as f64);

            weights.push((idx, w));

            sum += w;
        }

        for (_idx, w) in &mut weights {
            *w /= sum;
        }

        kernel.push(weights);
    }

    kernel
}

static KERNEL_CACHE: OnceLock<Mutex<HashMap<(usize, usize), Vec<Vec<(isize, f64)>>>>> =
    OnceLock::new();

fn get_or_compute_kernel(src: usize, dst: usize) -> Vec<Vec<(isize, f64)>> {
    let cache = KERNEL_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    let mut lock = cache.lock().unwrap();

    if let Some(k) = lock.get(&(src, dst)) {
        return k.clone();
    }

    let computed = generate_kernel(src, dst);

    lock.insert((src, dst), computed.clone());

    computed
}

fn resize_1d(
    src: ArrayView2<f64>,
    mut dst: ArrayViewMut2<f64>,
    kernel: &[Vec<(isize, f64)>],
    axis: Axis,
) {
    let out_len = dst.len_of(axis);

    let other_axis = axis.index() ^ 1;

    let in_len = src.len_of(axis);

    for out_i in 0..out_len {
        for j in 0..src.len_of(Axis(other_axis)) {
            let mut val = 0.0;

            let mut valid_weight_sum = 0.0;

            for &(src_i, weight) in &kernel[out_i] {
                if src_i >= 0 && (src_i as usize) < in_len {
                    let idx = if axis == Axis(0) {
                        [src_i as usize, j]
                    } else {
                        [j, src_i as usize]
                    };

                    if !src[idx].is_nan() {
                        val += weight * src[idx];

                        valid_weight_sum += weight;
                    }
                }
            }

            let idx = if axis == Axis(0) {
                [out_i, j]
            } else {
                [j, out_i]
            };

            dst[idx] = if valid_weight_sum >= 0.5 {
                val / valid_weight_sum
            } else {
                f64::NAN
            };
        }
    }
}

pub fn resize_lanczos3(input: &Array2<f64>, output_size: (usize, usize)) -> Array2<f64> {
    let (h_in, w_in) = input.dim();
    let (h_out, w_out) = output_size;

    let kernel_x = get_or_compute_kernel(w_in, w_out);
    let kernel_y = get_or_compute_kernel(h_in, h_out);

    let mut tmp = Array2::<f64>::zeros((h_in, w_out));
    resize_1d(input.view(), tmp.view_mut(), &kernel_x, Axis(1));

    let mut output = Array2::<f64>::zeros((h_out, w_out));
    resize_1d(tmp.view(), output.view_mut(), &kernel_y, Axis(0));

    output
}
