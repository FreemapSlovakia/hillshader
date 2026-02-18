use crate::{
    lanczos::resize_lanczos3,
    tilecodec::EDGE_BUFFER_PX,
};
use ndarray::{Array2, s};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OverviewResampling {
    Lanczos,
    Bilinear,
}

struct Resize {
    src: usize,
    dest: usize,
    size: usize,
}

pub fn compose_overview_tile(
    children: impl IntoIterator<Item = (usize, Array2<f64>)>,
    tile_size: usize,
    fill_value: f64,
    resampling: OverviewResampling,
) -> Option<Array2<f64>> {
    let children: Vec<_> = children.into_iter().collect();

    if children.is_empty() {
        return None;
    }

    let mut dem = Array2::<f64>::from_elem(
        (
            (tile_size + EDGE_BUFFER_PX * 2) * 2,
            (tile_size + EDGE_BUFFER_PX * 2) * 2,
        ),
        fill_value,
    );

    for (sector, child_dem) in children {
        let adjust = |c: usize| match c {
            0 => Resize {
                dest: 0,
                src: EDGE_BUFFER_PX + tile_size - 2 * EDGE_BUFFER_PX,
                size: 2 * EDGE_BUFFER_PX,
            },
            1 => Resize {
                dest: 2 * EDGE_BUFFER_PX,
                src: EDGE_BUFFER_PX,
                size: tile_size,
            },
            2 => Resize {
                dest: 2 * EDGE_BUFFER_PX + tile_size,
                src: EDGE_BUFFER_PX,
                size: tile_size,
            },
            3 => Resize {
                dest: 2 * EDGE_BUFFER_PX + 2 * tile_size,
                src: EDGE_BUFFER_PX,
                size: 2 * EDGE_BUFFER_PX,
            },
            _ => panic!("out of range"),
        };

        let y = adjust(sector & 3);
        let x = adjust(sector >> 2);

        dem.slice_mut(s![y.dest..(y.dest + y.size), x.dest..(x.dest + x.size)])
            .assign(&child_dem.slice(s![y.src..y.src + y.size, x.src..x.src + x.size]));
    }

    let out_size = (
        tile_size + EDGE_BUFFER_PX * 2,
        tile_size + EDGE_BUFFER_PX * 2,
    );

    Some(match resampling {
        OverviewResampling::Lanczos => resize_lanczos3(&dem, out_size),
        OverviewResampling::Bilinear => resize_bilinear(&dem, out_size),
    })
}

fn resize_bilinear(input: &Array2<f64>, output_size: (usize, usize)) -> Array2<f64> {
    let (h_in, w_in) = input.dim();
    let (h_out, w_out) = output_size;

    let mut output = Array2::<f64>::from_elem((h_out, w_out), f64::NAN);

    let scale_x = w_in as f64 / w_out as f64;
    let scale_y = h_in as f64 / h_out as f64;

    for oy in 0..h_out {
        let sy = (oy as f64 + 0.5) * scale_y - 0.5;
        let y0 = sy.floor().max(0.0) as usize;
        let y1 = (y0 + 1).min(h_in - 1);
        let wy = sy - y0 as f64;

        for ox in 0..w_out {
            let sx = (ox as f64 + 0.5) * scale_x - 0.5;
            let x0 = sx.floor().max(0.0) as usize;
            let x1 = (x0 + 1).min(w_in - 1);
            let wx = sx - x0 as f64;

            let samples = [
                (y0, x0, (1.0 - wx) * (1.0 - wy)),
                (y0, x1, wx * (1.0 - wy)),
                (y1, x0, (1.0 - wx) * wy),
                (y1, x1, wx * wy),
            ];

            let (sum, weight_sum) = samples.iter().fold((0.0, 0.0), |acc, (y, x, w)| {
                let v = input[[*y, *x]];
                if v.is_nan() {
                    acc
                } else {
                    (acc.0 + v * *w, acc.1 + *w)
                }
            });

            if weight_sum > 0.0 {
                output[[oy, ox]] = sum / weight_sum;
            }
        }
    }

    output
}
