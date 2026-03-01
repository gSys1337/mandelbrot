use crate::complex_plane::ComplexPlane;
use eframe::egui;
use eframe::egui::color_picker::{color_edit_button_srgba, Alpha};
use eframe::egui::{Color32, ColorImage, Rect, Sense, TextureFilter, TextureOptions};
use std::cmp::min;

mod complex_plane;

struct MandelbrotApp {
    /// This handle is used as a buffer for egui.
    texture_handle: egui::TextureHandle,
    /// After how many iterations the pixel is considered to be in the Mandelbrot set.
    max_iterations: usize,
    /// The color that numbers *outside* the Mandelbrot set are mapped to.
    color_start: Color32,
    /// The color that numbers *inside* the Mandelbrot set are mapped to.
    color_end: Color32,
    domain_history: Vec<Rect>,
    domain_index: usize,
    resolution_x: usize,
    resolution_y: usize,
    total_drag: Option<egui::Vec2>,
    drag_start: Option<egui::Pos2>,
    drag_end: Option<egui::Pos2>,
}

impl MandelbrotApp {
    const DEFAULT_ITERATIONS: usize = 13;
    const DEFAULT_COLOR_START: Color32 = Color32::GOLD;
    const DEFAULT_COLOR_END: Color32 = Color32::BLACK;
    const DEFAULT_DOMAIN: Rect = Rect::from_min_max(egui::pos2(-2.0, -2.0), egui::pos2(2.0, 2.0));

    /// Creates a new MandelbrotApp for eframe.
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let default_image = ColorImage::default();
        let handle = cc.egui_ctx.load_texture(
            "mandelbrot buffer",
            default_image,
            TextureOptions::default(),
        );
        let mut history = Vec::new();
        history.push(Self::DEFAULT_DOMAIN);
        Self {
            texture_handle: handle,
            resolution_x: 1,
            resolution_y: 1,
            max_iterations: Self::DEFAULT_ITERATIONS,
            color_start: Self::DEFAULT_COLOR_START,
            color_end: Self::DEFAULT_COLOR_END,
            domain_history: history,
            domain_index: 0,
            drag_end: None,
            drag_start: None,
            total_drag: None,
        }
    }

    /// Displays the image.
    fn image(&self) -> ColorImage {
        let raw_image: Vec<Color32> = ComplexPlane::new(
            self.domain_history
                .last()
                .copied()
                .unwrap_or(Self::DEFAULT_DOMAIN),
            [min(2048, self.resolution_x), min(2048, self.resolution_y)],
        )
        .generate_image(self.max_iterations)
        .iter()
        .copied()
        .map(|v| two_color_interpolation(self.color_start, self.color_end, v))
        .collect();
        ColorImage::new(
            [min(2048, self.resolution_x), min(2048, self.resolution_y)],
            raw_image,
        )
    }
}

impl eframe::App for MandelbrotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("left side panel").show(ctx, |ui| {
            ui.heading("Mandelbrot Viewer");
            ui.separator();

            ui.label(format!("Height: {}", self.resolution_y));
            ui.label(format!("Width: {}", self.resolution_x));

            let iterations = egui::Slider::new(&mut self.max_iterations, 1..=1000)
                .integer()
                .text("Iterations")
                .drag_value_speed(0.5);
            ui.add(iterations);

            color_edit_button_srgba(ui, &mut self.color_start, Alpha::Opaque);
            color_edit_button_srgba(ui, &mut self.color_end, Alpha::Opaque);
            ui.separator();

            ui.label(format!("Total Drag: {:?}", self.total_drag));
            ui.label(format!("Drag Start: {:?}", self.drag_start));
            ui.label(format!("Drag End: {:?}", self.drag_end));

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
                .maintain_aspect_ratio(false)
                .sense(Sense::all());
            let img_resp = ui.add(image);
            self.resolution_x = img_resp.rect.width() as usize;
            self.resolution_y = img_resp.rect.height() as usize;
            self.total_drag = img_resp.total_drag_delta();
            if img_resp.drag_started() {
                if let Some(pos) = img_resp.interact_pointer_pos()
                    && img_resp.rect.contains(pos)
                {
                    self.drag_start = Some(pos);
                } else {
                    self.drag_start = None;
                }
            }
            if img_resp.drag_stopped() {
                if let Some(pos_end) = img_resp.interact_pointer_pos()
                    && img_resp.rect.contains(pos_end)
                    && let Some(pos_start) = self.drag_start
                {
                    let new_domain = Rect::from_two_pos(pos_start, pos_end);
                    println!("New domain: {:?}", new_domain);
                }
            }
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
