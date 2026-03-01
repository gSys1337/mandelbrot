use eframe::egui;
use eframe::egui::pos2;
use num::complex::Complex64;
use std::cmp::min;
use std::collections::HashMap;

pub struct ComplexPlane(Vec<Complex64>);

impl ComplexPlane {
    /// Discretizes the domain into a grid with the given resolution.
    pub fn new(domain: egui::Rect, resolution: [usize; 2]) -> Self {
        let mut pixels: Vec<Complex64> = Vec::new();
        let delta_x = domain.width() / resolution[0] as f32;
        let delta_y = domain.height() / resolution[1] as f32;
        let mut y = domain.min.y;
        for _y in 0..resolution[1] {
            let mut x = domain.min.x;
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
    /// Discretizes the domain from [-2, -2] to [2, 2].
    pub fn new_around_origin(resolution: [usize; 2]) -> Self {
        Self::new(
            egui::Rect::from_min_max(pos2(-2.0, -2.0), pos2(2.0, 2.0)),
            resolution,
        )
    }

    /// Map the subset of the complex plane into a gray-scaled image.
    pub fn generate_image(self, max_iterations: usize) -> Vec<f64> {
        let ComplexPlane(pixels) = self;
        let mut iterations_counted = Vec::new();
        let mut histogram: HashMap<usize, usize> =
            HashMap::with_capacity(min(max_iterations, pixels.len()));
        // These are the usual mandelbrot operations
        for pixel_init in pixels {  // TODO make this loop concurrent with rayon crate
            let mut iterations = 0usize;
            let mut pixel = Complex64::new(0.0, 0.0);
            while pixel.norm_sqr() < 4.0 && iterations < max_iterations {
                pixel = pixel * pixel + pixel_init;
                iterations += 1;
            }
            iterations_counted.push(iterations);
            histogram
                .entry(iterations)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }
        // This is only coloring
        let total_iterations = histogram.values().sum::<usize>();
        let mut image = Vec::new();
        for pixel in iterations_counted {
            let hue = (0..=pixel)
                .map(|i| histogram.get(&i).copied().unwrap_or_default())
                .sum::<usize>() as f64
                / total_iterations as f64;
            image.push(hue);
        }
        image
    }
}
