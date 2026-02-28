use crate::complex_plane::ComplexPlane;
use eframe::egui;
use eframe::egui::{Color32, ColorImage, TextureFilter, TextureOptions};

mod complex_plane;

struct MandelbrotApp {
    /// This handle is used as a buffer for egui.
    texture_handle: egui::TextureHandle,
    /// After how many iterations the pixel is considered to be in the Mandelbrot set.
    max_iterations: usize,
    resolution_x: usize,
    resolution_y: usize,
    color_start: Color32,
    color_end: Color32,
}

impl MandelbrotApp {
    const DEFAULT_RESOLUTION_X: usize = 201;
    const DEFAULT_RESOLUTION_Y: usize = 101;
    const DEFAULT_ITERATIONS: usize = 13;
    const DEFAULT_COLOR_START: Color32 = Color32::GOLD;
    const DEFAULT_COLOR_END: Color32 = Color32::BLACK;

    /// Creates a new MandelbrotApp for eframe.
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let default_image = ColorImage::default();
        let handle = cc.egui_ctx.load_texture(
            "mandelbrot buffer",
            default_image,
            TextureOptions::default(),
        );
        Self {
            texture_handle: handle,
            resolution_x: Self::DEFAULT_RESOLUTION_X,
            resolution_y: Self::DEFAULT_RESOLUTION_Y,
            max_iterations: Self::DEFAULT_ITERATIONS,
            color_start: Self::DEFAULT_COLOR_START,
            color_end: Self::DEFAULT_COLOR_END,
        }
    }

    /// Displays the image.
    fn image(&self) -> ColorImage {
        let raw_image: Vec<Color32> =
            ComplexPlane::new_around_origin([self.resolution_x, self.resolution_y])
                .generate_image(self.max_iterations)
                .iter()
                .copied()
                .map(|v| two_color_interpolation(self.color_start, self.color_end, v))
                .collect();
        ColorImage::new([self.resolution_x, self.resolution_y], raw_image)
    }
}

impl eframe::App for MandelbrotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("left side panel").show(ctx, |ui| {
            ui.heading("Mandelbrot Viewer");
            ui.separator();

            let resolution_x = egui::Slider::new(&mut self.resolution_x, 1..=2048)
                .integer()
                .text("Width")
                .drag_value_speed(1.0);
            ui.add(resolution_x);
            let resolution_y = egui::Slider::new(&mut self.resolution_y, 1..=2048)
                .integer()
                .text("Height")
                .drag_value_speed(1.0);
            ui.add(resolution_y);
            let iterations = egui::Slider::new(&mut self.max_iterations, 1..=1000)
                .integer()
                .text("Iterations")
                .drag_value_speed(0.5);
            ui.add(iterations);

            ui.separator();
            if ui.button("Generate image").clicked() {
                self.texture_handle.set(
                    self.image(),
                    TextureOptions {
                        magnification: TextureFilter::Nearest,
                        ..Default::default()
                    },
                );
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let image = egui::Image::from_texture(&self.texture_handle)
                .shrink_to_fit()
                .maintain_aspect_ratio(false);
            ui.add(image);
        });
    }
}

fn main() -> eframe::Result {
    let viewport_builder = egui::ViewportBuilder::default().with_title("Mandelbrot Viewer");
    eframe::run_native(
        "Mandelbrot",
        eframe::NativeOptions {
            viewport: viewport_builder,
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(MandelbrotApp::new(cc)))),
    )
}

fn two_color_interpolation(start: Color32, end: Color32, fraction: f64) -> Color32 {
    let add = u8::wrapping_add;
    let sub = u8::wrapping_sub;
    Color32::from_rgba_premultiplied(
        add(start.r(), (sub(end.r(), start.r()) as f64 * fraction) as u8),
        add(start.g(), (sub(end.g(), start.g()) as f64 * fraction) as u8),
        add(start.b(), (sub(end.b(), start.b()) as f64 * fraction) as u8),
        add(start.a(), (sub(end.a(), start.a()) as f64 * fraction) as u8),
    )
}
