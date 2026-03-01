use crate::complex_plane::ComplexPlane;
use crate::history::Domain;
use eframe::egui;
use eframe::egui::color_picker::{Alpha, color_edit_button_srgba};
use eframe::egui::{Color32, ColorImage, Rect, Sense, TextureFilter, TextureOptions};
use std::cmp::min;
use std::sync::mpsc::{self, Receiver};

enum CalculationAction {
    ReplaceLast,
    PushNewZoom,
    Reset,
}

struct CalculationResult {
    rect: Rect,
    gray_image: Vec<f64>,
    raw_image: Vec<Color32>,
    size: [usize; 2],
    action: CalculationAction,
}

mod complex_plane;
mod history;

struct MandelbrotApp {
    /// After how many iterations the pixel is considered to be in the Mandelbrot set.
    max_iterations: usize,
    /// The color that numbers *outside* the Mandelbrot set are mapped to.
    color_start: Color32,
    /// The color that numbers *inside* the Mandelbrot set are mapped to.
    color_end: Color32,
    domain_history: Vec<Domain>,
    domain_future: Vec<Domain>,
    resolution_x: usize,
    resolution_y: usize,
    drag_start: Option<egui::Pos2>,
    calculation_receiver: Option<Receiver<CalculationResult>>,
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
        let history = vec![Domain {
            rect: Self::DEFAULT_DOMAIN,
            gray_image: Vec::new(),
            texture: handle,
        }];
        Self {
            resolution_x: 1,
            resolution_y: 1,
            max_iterations: Self::DEFAULT_ITERATIONS,
            color_start: Self::DEFAULT_COLOR_START,
            color_end: Self::DEFAULT_COLOR_END,
            domain_history: history,
            domain_future: Vec::new(),
            drag_start: None,
            calculation_receiver: None,
        }
    }

    fn start_calculation(&mut self, ctx: &egui::Context, rect: Rect, action: CalculationAction) {
        let (tx, rx) = mpsc::channel();
        self.calculation_receiver = Some(rx);

        let ctx_clone = ctx.clone();
        let size = [min(2048, self.resolution_x), min(2048, self.resolution_y)];
        let max_iters = self.max_iterations;
        let c_start = self.color_start;
        let c_end = self.color_end;

        std::thread::spawn(move || {
            let gray_image = ComplexPlane::new(rect, size).generate_image(max_iters);

            let raw_image: Vec<Color32> = gray_image
                .iter()
                .copied()
                .map(|v| two_color_interpolation(c_start, c_end, v))
                .collect();

            let result = CalculationResult {
                rect,
                gray_image,
                raw_image,
                size,
                action,
            };

            let _ = tx.send(result);
            ctx_clone.request_repaint();
        });
    }

    // TODO: make this function a method of Domain
    fn apply_colors(domain: &mut Domain, color_start: Color32, color_end: Color32) {
        if domain.gray_image.is_empty() {
            return;
        }
        let size = domain.texture.size();
        let raw_image: Vec<Color32> = domain
            .gray_image
            .iter()
            .copied()
            .map(|v| two_color_interpolation(color_start, color_end, v))
            .collect();
        let color_image = ColorImage::new(size, raw_image);
        domain.texture.set(
            color_image,
            TextureOptions {
                magnification: TextureFilter::Nearest,
                ..Default::default()
            },
        );
    }
}

impl eframe::App for MandelbrotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.calculation_receiver {
            if let Ok(result) = rx.try_recv() {
                let color_image = ColorImage::new(result.size, result.raw_image);
                let texture = ctx.load_texture(
                    "mandelbrot buffer",
                    color_image,
                    TextureOptions {
                        magnification: TextureFilter::Nearest,
                        ..Default::default()
                    },
                );

                let new_domain = Domain {
                    rect: result.rect,
                    gray_image: result.gray_image,
                    texture,
                };

                match result.action {
                    CalculationAction::Reset => {
                        self.domain_history.clear();
                        self.domain_future.clear();
                        self.domain_history.push(new_domain);
                    }
                    CalculationAction::PushNewZoom => {
                        self.domain_future.clear();
                        self.domain_history.push(new_domain);
                    }
                    CalculationAction::ReplaceLast => {
                        if let Some(domain) = self.domain_history.last_mut() {
                            *domain = new_domain;
                        } else {
                            self.domain_history.push(new_domain);
                        }
                    }
                }

                self.calculation_receiver = None;
            }
        }

        let is_calculating = self.calculation_receiver.is_some();

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

            let mut colors_changed = false;
            colors_changed |=
                color_edit_button_srgba(ui, &mut self.color_start, Alpha::Opaque).changed();
            colors_changed |=
                color_edit_button_srgba(ui, &mut self.color_end, Alpha::Opaque).changed();
            ui.separator();

            if colors_changed && let Some(domain) = self.domain_history.last_mut() {
                let color_start = self.color_start;
                let color_end = self.color_end;
                Self::apply_colors(domain, color_start, color_end);
            }

            if let Some(domain) = self.domain_history.last() {
                ui.label("Current domain:");
                ui.label(format!("Min: {:.6?}", domain.rect.min));
                ui.label(format!("Max: {:.6?}", domain.rect.max));
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !is_calculating && self.domain_history.len() > 1,
                        egui::Button::new("Previous"),
                    )
                    .clicked()
                {
                    if let Some(domain) = self.domain_history.pop() {
                        self.domain_future.push(domain);
                    }
                    if let Some(domain) = self.domain_history.last_mut() {
                        let color_start = self.color_start;
                        let color_end = self.color_end;
                        Self::apply_colors(domain, color_start, color_end);
                    }
                }
                if ui
                    .add_enabled(
                        !is_calculating && !self.domain_future.is_empty(),
                        egui::Button::new("Next"),
                    )
                    .clicked()
                {
                    if let Some(domain) = self.domain_future.pop() {
                        self.domain_history.push(domain);
                    }
                    if let Some(domain) = self.domain_history.last_mut() {
                        let color_start = self.color_start;
                        let color_end = self.color_end;
                        Self::apply_colors(domain, color_start, color_end);
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!is_calculating, egui::Button::new("Generate image"))
                    .clicked()
                {
                    let rect = self
                        .domain_history
                        .last()
                        .map(|d| d.rect)
                        .unwrap_or(Self::DEFAULT_DOMAIN);
                    self.start_calculation(ctx, rect, CalculationAction::ReplaceLast);
                }
                if is_calculating {
                    ui.spinner();
                }
            });
            if ui
                .add_enabled(!is_calculating, egui::Button::new("Reset"))
                .clicked()
            {
                self.start_calculation(ctx, Self::DEFAULT_DOMAIN, CalculationAction::Reset);
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            let current_texture = &self
                .domain_history
                .last()
                .expect("History should never be empty")
                .texture;
            let image = egui::Image::from_texture(current_texture)
                .shrink_to_fit()
                .maintain_aspect_ratio(false)
                .sense(Sense::all());
            let img_resp = ui.add(image);
            self.resolution_x = img_resp.rect.width() as usize;
            self.resolution_y = img_resp.rect.height() as usize;
            if img_resp.drag_started() && !is_calculating {
                if let Some(pos) = img_resp.interact_pointer_pos()
                    && img_resp.rect.contains(pos)
                {
                    self.drag_start = Some(pos);
                } else {
                    self.drag_start = None;
                }
            }
            if img_resp.drag_stopped()
                && !is_calculating
                && let Some(pos_end) = img_resp.interact_pointer_pos()
                && img_resp.rect.contains(pos_end)
                && let Some(pos_start) = self.drag_start
            {
                let current_domain_rect = self
                    .domain_history
                    .last()
                    .map(|d| d.rect)
                    .unwrap_or(Self::DEFAULT_DOMAIN);

                let map_to_complex = |pos: egui::Pos2| -> egui::Pos2 {
                    let x = current_domain_rect.min.x
                        + (pos.x - img_resp.rect.min.x) / img_resp.rect.width()
                            * current_domain_rect.width();
                    let y = current_domain_rect.min.y
                        + (pos.y - img_resp.rect.min.y) / img_resp.rect.height()
                            * current_domain_rect.height();
                    egui::pos2(x, y)
                };

                let new_domain =
                    Rect::from_two_pos(map_to_complex(pos_start), map_to_complex(pos_end));

                if new_domain.width() > 0.0 && new_domain.height() > 0.0 {
                    self.start_calculation(ctx, new_domain, CalculationAction::PushNewZoom);
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
