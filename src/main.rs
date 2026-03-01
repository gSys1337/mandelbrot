use eframe::egui;
use eframe::egui::color_picker::{Alpha, color_edit_button_srgba};
use eframe::egui::{Color32, ColorImage, Rect, Sense, TextureFilter, TextureOptions};
use mandelbrot_test::mandel_set::{Codomain, Domain};
use mandelbrot_test::two_color_interpolation;
use std::cmp::min;
use std::sync::mpsc::{self, Receiver};

enum CalculationAction {
    ReplaceLast,
    PushNewZoom,
    Reset,
}

struct CalculationResult {
    rect: Rect,
    image: Vec<f64>,
    raw_image: Vec<Color32>,
    size: [usize; 2],
    action: CalculationAction,
}

struct MandelbrotApp {
    /// After how many iterations the pixel is considered to be in the Mandelbrot set.
    max_iterations: usize,
    /// The color that numbers *outside* the Mandelbrot set are mapped to.
    color_start: Color32,
    /// The color that numbers *inside* the Mandelbrot set are mapped to.
    color_end: Color32,
    codomain_history: Vec<Codomain>,
    codomain_future: Vec<Codomain>,
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
        let history = vec![Codomain {
            rect: Self::DEFAULT_DOMAIN,
            image: Vec::new(),
            texture: handle,
        }];
        Self {
            resolution_x: 1,
            resolution_y: 1,
            max_iterations: Self::DEFAULT_ITERATIONS,
            color_start: Self::DEFAULT_COLOR_START,
            color_end: Self::DEFAULT_COLOR_END,
            codomain_history: history,
            codomain_future: Vec::new(),
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
            let image = Domain::new(rect, size).generate_image(max_iters);

            let raw_image: Vec<Color32> = image
                .iter()
                .copied()
                .map(|v| two_color_interpolation(c_start, c_end, v))
                .collect();

            let result = CalculationResult {
                rect,
                image,
                raw_image,
                size,
                action,
            };

            let _ = tx.send(result);
            ctx_clone.request_repaint();
        });
    }

    fn handle_calculation_result(&mut self, ctx: &egui::Context, result: CalculationResult) {
        let color_image = ColorImage::new(result.size, result.raw_image);
        let texture = ctx.load_texture(
            "mandelbrot buffer",
            color_image,
            TextureOptions {
                magnification: TextureFilter::Nearest,
                ..Default::default()
            },
        );

        let new_codomain = Codomain {
            rect: result.rect,
            image: result.image,
            texture,
        };

        match result.action {
            CalculationAction::Reset => {
                self.codomain_history.clear();
                self.codomain_future.clear();
                self.codomain_history.push(new_codomain);
            }
            CalculationAction::PushNewZoom => {
                self.codomain_future.clear();
                self.codomain_history.push(new_codomain);
            }
            CalculationAction::ReplaceLast => {
                if let Some(codomain) = self.codomain_history.last_mut() {
                    *codomain = new_codomain;
                } else {
                    self.codomain_history.push(new_codomain);
                }
            }
        }
        self.calculation_receiver = None;
    }

    fn show_left_sidepanel(&mut self, ctx: &egui::Context) {
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

            if colors_changed && let Some(codomain) = self.codomain_history.last_mut() {
                let color_start = self.color_start;
                let color_end = self.color_end;
                codomain.apply_colors(color_start, color_end);
            }

            if let Some(codomain) = self.codomain_history.last() {
                ui.label("Current restriction:");
                ui.label(format!("Min: {:.6?}", codomain.rect.min));
                ui.label(format!("Max: {:.6?}", codomain.rect.max));
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.calculation_receiver.is_none() && self.codomain_history.len() > 1,
                        egui::Button::new("Previous"),
                    )
                    .clicked()
                {
                    if let Some(codomain) = self.codomain_history.pop() {
                        self.codomain_future.push(codomain);
                    }
                    if let Some(codomain) = self.codomain_history.last_mut() {
                        let color_start = self.color_start;
                        let color_end = self.color_end;
                        codomain.apply_colors(color_start, color_end);
                    }
                }
                if ui
                    .add_enabled(
                        self.calculation_receiver.is_none() && !self.codomain_future.is_empty(),
                        egui::Button::new("Next"),
                    )
                    .clicked()
                {
                    if let Some(codomain) = self.codomain_future.pop() {
                        self.codomain_history.push(codomain);
                    }
                    if let Some(codomain) = self.codomain_history.last_mut() {
                        let color_start = self.color_start;
                        let color_end = self.color_end;
                        codomain.apply_colors(color_start, color_end);
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.calculation_receiver.is_none(),
                        egui::Button::new("Generate image"),
                    )
                    .clicked()
                {
                    let rect = self
                        .codomain_history
                        .last()
                        .map(|d| d.rect)
                        .unwrap_or(Self::DEFAULT_DOMAIN);
                    self.start_calculation(ctx, rect, CalculationAction::ReplaceLast);
                }
                if self.calculation_receiver.is_some() {
                    ui.spinner();
                }
            });
            if ui
                .add_enabled(
                    self.calculation_receiver.is_none(),
                    egui::Button::new("Reset"),
                )
                .clicked()
            {
                self.start_calculation(ctx, Self::DEFAULT_DOMAIN, CalculationAction::Reset);
            }
        });
    }

    fn show_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let current_texture = &self
                .codomain_history
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
            if img_resp.drag_started() && self.calculation_receiver.is_none() {
                if let Some(pos) = img_resp.interact_pointer_pos()
                    && img_resp.rect.contains(pos)
                {
                    self.drag_start = Some(pos);
                } else {
                    self.drag_start = None;
                }
            }
            if img_resp.drag_stopped()
                && self.calculation_receiver.is_none()
                && let Some(pos_end) = img_resp.interact_pointer_pos()
                && img_resp.rect.contains(pos_end)
                && let Some(pos_start) = self.drag_start
            {
                let current_domain_rect = self
                    .codomain_history
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

                let new_restriction =
                    Rect::from_two_pos(map_to_complex(pos_start), map_to_complex(pos_end));

                if new_restriction.width() > 0.0 && new_restriction.height() > 0.0 {
                    self.start_calculation(ctx, new_restriction, CalculationAction::PushNewZoom);
                }
            }
        });
    }
}

impl eframe::App for MandelbrotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.calculation_receiver
            && let Ok(result) = rx.try_recv()
        {
            self.handle_calculation_result(ctx, result);
        }
        self.show_left_sidepanel(ctx);
        self.show_central_panel(ctx);
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
