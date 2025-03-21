use crate::shared_types::{
    IgorShadingParams, ObliqueShadingParams, Shading, ShadingMethod, SlopeShadingParams,
};
use image::RgbImage;
use std::f64;

pub fn compute_hillshade<F>(
    elevation: &[f64],
    z_factor: f64,
    rows: usize,
    cols: usize,
    compute_rgb: F,
) -> RgbImage
where
    F: Fn(f64, f64) -> [u8; 3],
{
    let mut hillshade = RgbImage::new(cols as u32, rows as u32);

    for y in 1..rows - 1 {
        for x in 1..cols - 1 {
            let (slope_rad, aspect_rad) = compute_slope_and_aspect(elevation, z_factor, cols, x, y);

            hillshade.get_pixel_mut(x as u32, (rows - y) as u32).0 =
                compute_rgb(aspect_rad, slope_rad);
        }
    }

    hillshade
}

fn compute_slope_and_aspect(
    elevation: &[f64],
    z_factor: f64,
    cols: usize,
    x: usize,
    y: usize,
) -> (f64, f64) {
    let off = y * cols;

    // Extract 3x3 window
    let z1 = elevation[off - cols + x - 1];
    let z2 = elevation[off - cols + x];
    let z3 = elevation[off - cols + x + 1];
    let z4 = elevation[off + x - 1];
    let z6 = elevation[off + x + 1];
    let z7 = elevation[off + cols + x - 1];
    let z8 = elevation[off + cols + x];
    let z9 = elevation[off + cols + x + 1];

    // Compute raw derivatives (Horn method)
    let dz_dx = (-z1 + z3 - 2.0 * z4 + 2.0 * z6 - z7 + z9) / 8.0;
    let dz_dy = (-z1 - 2.0 * z2 - z3 + z7 + 2.0 * z8 + z9) / 8.0;

    // Apply z-factor
    let dz_dx = dz_dx * z_factor;
    let dz_dy = dz_dy * z_factor;

    // Compute slope
    let mut slope_rad = dz_dx.hypot(dz_dy).atan();

    // Compute aspect
    let mut aspect_rad = dz_dy.atan2(-dz_dx);

    if aspect_rad < 0.0 {
        aspect_rad += std::f64::consts::TAU;
    }

    if aspect_rad.is_nan() || slope_rad.is_nan() {
        slope_rad = 0.0;
        aspect_rad = 0.0;
    }

    (slope_rad, aspect_rad)
}

pub fn shade(
    aspect_rad: f64,
    slope_rad: f64,
    shadings: &[Shading],
    contrast: f64,
    brightness: f64,
) -> [u8; 3] {
    // Compute modified hillshade values
    let mods: Vec<_> = shadings
        .iter()
        .map(|shading| {
            let value = match &shading.method {
                ShadingMethod::Igor(IgorShadingParams { azimuth }) => {
                    let aspect_diff = difference_between_angles(
                        aspect_rad,
                        f64::consts::PI * 1.5 - azimuth.to_radians(),
                        f64::consts::PI * 2.0,
                    );

                    let aspect_strength = 1.0 - aspect_diff / f64::consts::PI;

                    1.0 - slope_rad * 2.0 * aspect_strength
                }
                ShadingMethod::Oblique(ObliqueShadingParams { azimuth, altitude }) => {
                    let zenith = f64::consts::FRAC_PI_2 - altitude;

                    (zenith).cos() * slope_rad.cos()
                        + (zenith).sin() * slope_rad.sin() * (azimuth - aspect_rad).cos()
                }
                ShadingMethod::Slope(SlopeShadingParams { altitude }) => {
                    let zenith = f64::consts::FRAC_PI_2 - altitude;

                    (zenith).cos() * slope_rad.cos() + (zenith).sin() * slope_rad.sin()
                }
            };

            ((shading.color & 0xFF) as f64 / 255.0) * (1.0 - value)
        })
        .collect();

    // Normalization factor
    let norm = f64::MIN_POSITIVE + mods.iter().sum::<f64>();

    let alpha = 1.0 - mods.iter().map(|m| 1.0 - m).product::<f64>();

    // Compute each channel
    let compute_channel = |shift| {
        let sum: f64 = mods
            .iter()
            .enumerate()
            .map(|(i, m)| m * f64::from((shadings[i].color >> shift) & 0xFF_u32) / 255.0)
            .sum();

        let value = contrast * ((sum / norm) - 0.5) + 0.5 + brightness;

        let value = value + (1.0 - value) * (1.0 - alpha);

        (value * 255.0).clamp(0.0, 255.0) as u8
    };

    let r = compute_channel(24);
    let g = compute_channel(16);
    let b = compute_channel(8);

    [r, g, b]
}

fn normalize_angle(angle: f64, normalizer: f64) -> f64 {
    let angle = angle % normalizer;

    if angle < 0.0 {
        normalizer + angle
    } else {
        angle
    }
}

fn difference_between_angles(angle1: f64, angle2: f64, normalizer: f64) -> f64 {
    let diff = (normalize_angle(angle1, normalizer) - normalize_angle(angle2, normalizer)).abs();

    if diff > normalizer / 2.0 {
        normalizer - diff
    } else {
        diff
    }
}
