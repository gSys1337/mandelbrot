use eframe::egui::{Color32, ColorImage, TextureFilter, TextureHandle, TextureOptions};
use num::FromPrimitive;
use num::{Complex, Zero};
use rayon::prelude::*;
use std::cmp::min;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Restriction {
    min: Complex<f64>,
    max: Complex<f64>,
    resolution_re: usize,
    resolution_im: usize,
}

impl Default for Restriction {
    fn default() -> Self {
        Self::from_two_points(Complex::new(-2.0, -2.0), Complex::new(2.0, 2.0), 800, 600)
    }
}

impl Restriction {
    /// Returns the bounding rectangle of the two points.
    #[inline]
    pub fn from_two_points(
        a: Complex<f64>,
        b: Complex<f64>,
        resolution_re: usize,
        resolution_im: usize,
    ) -> Self {
        Self {
            min: Complex {
                re: a.re.min(b.re),
                im: a.im.min(b.im),
            },
            max: Complex {
                re: a.re.max(b.re),
                im: a.im.max(b.im),
            },
            resolution_re: min(resolution_re, 2048usize),
            resolution_im: min(resolution_im, 2048usize),
        }
    }
    /// Discretizes the restriction into a grid with the given resolution.
    pub fn into_domain(self) -> Domain {
        let mut preimage: Vec<Complex<f64>> = Vec::new();
        let res_re = f64::from_usize(self.resolution_re).unwrap();
        let res_im = f64::from_usize(self.resolution_im).unwrap();
        let delta_re = (self.max.re - self.min.re) / res_re;
        let delta_im = (self.max.im - self.min.im) / res_im;
        let mut im = self.min.im;
        for _im in 0..self.resolution_im {
            let mut re = self.min.re;
            for _re in 0..self.resolution_re {
                let pixel = Complex::new(re, im);
                preimage.push(pixel);
                re += delta_re;
            }
            im += delta_im;
        }

        Domain {
            restriction: self,
            preimage,
        }
    }
    pub fn size(&self) -> [usize; 2] {
        [self.resolution_re, self.resolution_im]
    }
    pub fn width(&self) -> f64 {
        self.max.re - self.min.re
    }
    pub fn height(&self) -> f64 {
        self.max.im - self.min.im
    }
    pub fn min(&self) -> Complex<f64> {
        self.min
    }
    pub fn max(&self) -> Complex<f64> {
        self.max
    }
}

#[derive(Clone)]
pub struct Domain {
    restriction: Restriction,
    preimage: Vec<Complex<f64>>,
}

impl Domain {
    /// Map the subset of the complex plane into a gray-scaled image.
    pub fn calculate_image(self, max_iterations: usize) -> Codomain {
        self.calculate_image_by_rayon(max_iterations, true)
    }

    pub fn calculate_image_by_rayon(self, max_iterations: usize, use_rayon: bool) -> Codomain {
        let Domain {
            restriction,
            preimage,
        } = self;

        // Calculate the mandelbrot set
        let mandel_calc_loop = |pixel_init: Complex<f64>| {
            let mut iterations = 0usize;
            let mut pixel = Complex::<f64>::zero();
            while pixel.norm_sqr() < 4.0 && iterations < max_iterations {
                pixel = pixel.powi(2) + pixel_init;
                iterations += 1;
            }
            iterations
        };
        let number_iterations: Vec<usize> = if use_rayon {
            preimage.into_par_iter().map(mandel_calc_loop).collect()
        } else {
            preimage.into_iter().map(mandel_calc_loop).collect()
        };

        let mut histogram: HashMap<usize, usize> =
            HashMap::with_capacity(min(max_iterations, number_iterations.len()));

        for &iterations in &number_iterations {
            histogram
                .entry(iterations)
                .and_modify(|count| *count += 1)
                .or_insert(1);
        }

        // This is only coloring
        let total_iterations = histogram.values().sum::<usize>();
        let mandel_color_loop = |pixel: usize| {
            (0..=pixel)
                .map(|i| histogram.get(&i).copied().unwrap_or_default())
                .sum::<usize>() as f64
                / total_iterations as f64
        };
        let image = if use_rayon {
            number_iterations
                .into_par_iter()
                .map(mandel_color_loop)
                .collect()
        } else {
            number_iterations
                .into_iter()
                .map(mandel_color_loop)
                .collect()
        };

        Codomain {
            restriction,
            _max_iterations: max_iterations,
            image,
        }
    }
    pub fn restriction(&self) -> &Restriction {
        &self.restriction
    }
}

pub struct Codomain {
    restriction: Restriction,
    _max_iterations: usize,
    image: Vec<f64>,
}

impl Codomain {
    pub fn write_image_to_texture(
        &mut self,
        color_start: Color32,
        color_end: Color32,
        texture: &mut TextureHandle,
    ) {
        if self.image.is_empty() {
            return;
        }
        let size = texture.size();
        let raw_image: Vec<Color32> = self
            .image
            .iter()
            .copied()
            .map(|v| self::two_color_interpolation(color_start, color_end, v))
            .collect();
        let color_image = ColorImage::new(size, raw_image);
        texture.set(
            color_image,
            TextureOptions {
                magnification: TextureFilter::Nearest,
                ..Default::default()
            },
        );
    }

    pub fn iter(&self) -> impl Iterator<Item = f64> {
        self.image.iter().copied()
    }
    pub fn restriction(&self) -> &Restriction {
        &self.restriction
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
