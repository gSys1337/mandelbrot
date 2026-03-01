use eframe::egui;
use eframe::egui::pos2;
use eframe::egui::{Color32, ColorImage, Rect, TextureFilter, TextureHandle, TextureOptions};
use num::complex::Complex64;
use rayon::prelude::*;
use std::cmp::min;
use std::collections::HashMap;

pub struct Domain(Vec<Complex64>);

impl Domain {
    /// Discretizes the restriction into a grid with the given resolution.
    pub fn new(restriction: egui::Rect, resolution: [usize; 2]) -> Self {
        let mut pixels: Vec<Complex64> = Vec::new();
        let delta_x = restriction.width() / resolution[0] as f32;
        let delta_y = restriction.height() / resolution[1] as f32;
        let mut y = restriction.min.y;
        for _y in 0..resolution[1] {
            let mut x = restriction.min.x;
            for _x in 0..resolution[0] {
                let pixel = Complex64::new(x as f64, y as f64);
                pixels.push(pixel);
                x += delta_x;
            }
            y += delta_y;
        }
        Self(pixels)
    }

    #[allow(dead_code)]
    /// Discretizes the restriction from [-2, -2] to [2, 2].
    pub fn new_around_origin(resolution: [usize; 2]) -> Self {
        Self::new(
            egui::Rect::from_min_max(pos2(-2.0, -2.0), pos2(2.0, 2.0)),
            resolution,
        )
    }

    /// Map the subset of the complex plane into a gray-scaled image.
    pub fn generate_image(self, max_iterations: usize) -> Vec<f64> {
        self.generate_image_by_rayon(max_iterations, true)
    }

    pub fn generate_image_by_rayon(self, max_iterations: usize, use_rayon: bool) -> Vec<f64> {
        let Domain(pixels) = self;

        // Calculate the mandelbrot set
        let iterations_counted: Vec<usize> = if use_rayon {
            pixels
                .into_par_iter()
                .map(|pixel_init| {
                    let mut iterations = 0usize;
                    let mut pixel = Complex64::new(0.0, 0.0);
                    while pixel.norm_sqr() < 4.0 && iterations < max_iterations {
                        pixel = pixel * pixel + pixel_init;
                        iterations += 1;
                    }
                    iterations
                })
                .collect()
        } else {
            pixels
                .into_iter()
                .map(|pixel_init| {
                    let mut iterations = 0usize;
                    let mut pixel = Complex64::new(0.0, 0.0);
                    while pixel.norm_sqr() < 4.0 && iterations < max_iterations {
                        pixel = pixel * pixel + pixel_init;
                        iterations += 1;
                    }
                    iterations
                })
                .collect()
        };

        let mut histogram: HashMap<usize, usize> =
            HashMap::with_capacity(min(max_iterations, iterations_counted.len()));

        for &iterations in &iterations_counted {
            histogram
                .entry(iterations)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }

        // This is only coloring
        let total_iterations = histogram.values().sum::<usize>();
        if use_rayon {
            iterations_counted
                .into_par_iter()
                .map(|pixel| {
                    (0..=pixel)
                        .map(|i| histogram.get(&i).copied().unwrap_or_default())
                        .sum::<usize>() as f64
                        / total_iterations as f64
                })
                .collect()
        } else {
            iterations_counted
                .into_iter()
                .map(|pixel| {
                    (0..=pixel)
                        .map(|i| histogram.get(&i).copied().unwrap_or_default())
                        .sum::<usize>() as f64
                        / total_iterations as f64
                })
                .collect()
        }
    }
}

pub struct Codomain {
    pub rect: Rect,
    pub image: Vec<f64>,
    pub texture: TextureHandle,
}

impl Codomain {
    pub fn apply_colors(&mut self, color_start: Color32, color_end: Color32) {
        if self.image.is_empty() {
            return;
        }
        let size = self.texture.size();
        let raw_image: Vec<Color32> = self
            .image
            .iter()
            .copied()
            .map(|v| self::two_color_interpolation(color_start, color_end, v))
            .collect();
        let color_image = ColorImage::new(size, raw_image);
        self.texture.set(
            color_image,
            TextureOptions {
                magnification: TextureFilter::Nearest,
                ..Default::default()
            },
        );
    }
}

pub fn two_color_interpolation(start: Color32, end: Color32, fraction: f64) -> Color32 {
    let add = u8::wrapping_add;
    let sub = u8::wrapping_sub;
    Color32::from_rgba_premultiplied(
        add(start.r(), (sub(end.r(), start.r()) as f64 * fraction) as u8),
        add(start.g(), (sub(end.g(), start.g()) as f64 * fraction) as u8),
        add(start.b(), (sub(end.b(), start.b()) as f64 * fraction) as u8),
        add(start.a(), (sub(end.a(), start.a()) as f64 * fraction) as u8),
    )
}
