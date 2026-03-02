use eframe::egui;
use eframe::egui::color_picker::{Alpha, color_edit_button_srgba};
use eframe::egui::{Color32, ColorImage, Sense, TextureFilter, TextureOptions};
use num::Complex;
use std::sync::mpsc::{self, Receiver};

pub mod mandel_set;
use mandel_set::{Codomain, Restriction, two_color_interpolation};

pub enum CalculationAction {
    ReplaceLast,
    PushNewZoom,
    Reset,
}

pub struct CalculationResult {
    codomain: Codomain,
    raw_image: Vec<Color32>,
    action: CalculationAction,
}

pub struct MandelbrotApp {
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
    texture_handle: egui::TextureHandle,
    drag_start: Option<egui::Pos2>,
    calculation_receiver: Option<Receiver<CalculationResult>>,
}

impl MandelbrotApp {
    const DEFAULT_ITERATIONS: usize = 2;
    const DEFAULT_COLOR_START: Color32 = Color32::GOLD;
    const DEFAULT_COLOR_END: Color32 = Color32::BLACK;

    /// Creates a new MandelbrotApp for eframe.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let default_domain = Restriction::default()
            .into_domain()
            .calculate_image(Self::DEFAULT_ITERATIONS);
        let handle = cc.egui_ctx.load_texture(
            "mandelbrot buffer",
            ColorImage::default(),
            TextureOptions::default(),
        );
        // default_domain.write_image_to_texture(
        //     Self::DEFAULT_COLOR_START,
        //     Self::DEFAULT_COLOR_END,
        //     &mut handle,
        // );
        let history = vec![default_domain];
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
            texture_handle: handle,
        }
    }

    pub fn calculate_mandelbrot_set(
        &mut self,
        ctx: &egui::Context,
        restriction: Restriction,
        action: CalculationAction,
    ) {
        let (tx, rx) = mpsc::channel();
        self.calculation_receiver = Some(rx);

        let ctx_clone = ctx.clone();
        let max_iters = self.max_iterations;
        let c_start = self.color_start;
        let c_end = self.color_end;

        std::thread::spawn(move || {
            let image = restriction.into_domain().calculate_image(max_iters);

            let raw_image: Vec<Color32> = image
                .iter()
                .map(|v| two_color_interpolation(c_start, c_end, v))
                .collect();

            let result = CalculationResult {
                codomain: image,
                raw_image,
                action,
            };

            let _ = tx.send(result);
            ctx_clone.request_repaint();
        });
    }

    fn handle_calculation_result(&mut self, result: CalculationResult) {
        let color_image = ColorImage::new(result.codomain.restriction().size(), result.raw_image);
        self.texture_handle.set(
            color_image,
            TextureOptions {
                magnification: TextureFilter::Nearest,
                ..Default::default()
            },
        );

        match result.action {
            CalculationAction::Reset => {
                self.codomain_history.clear();
                self.codomain_future.clear();
                self.codomain_history.push(result.codomain);
            }
            CalculationAction::PushNewZoom => {
                self.codomain_future.clear();
                self.codomain_history.push(result.codomain);
            }
            CalculationAction::ReplaceLast => {
                if let Some(codomain) = self.codomain_history.last_mut() {
                    *codomain = result.codomain;
                } else {
                    self.codomain_history.push(result.codomain);
                }
            }
        }
        self.calculation_receiver = None;
    }

    fn show_left_sidepanel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left side panel").show(ctx, |ui| {
            ui.heading("Mandelbrot Viewer");
            ui.separator();

            ui.label("Resolution:");
            ui.label(format!("Horizontal: {}", self.resolution_x));
            ui.label(format!("Vertical: {}", self.resolution_y));

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
                codomain.write_image_to_texture(color_start, color_end, &mut self.texture_handle);
            }

            if let Some(codomain) = self.codomain_history.last() {
                ui.label("Current restriction:");
                ui.label(format!("Width: {}", codomain.restriction().width()));
                ui.label(format!("Height: {}", codomain.restriction().height()));
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
                        codomain.write_image_to_texture(
                            color_start,
                            color_end,
                            &mut self.texture_handle,
                        );
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
                        codomain.write_image_to_texture(
                            color_start,
                            color_end,
                            &mut self.texture_handle,
                        );
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
                    let restriction = self
                        .codomain_history
                        .last()
                        .map(|d| d.restriction())
                        .cloned()
                        .unwrap_or(Restriction::default());
                    self.calculate_mandelbrot_set(
                        ctx,
                        restriction.clone(),
                        CalculationAction::ReplaceLast,
                    );
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
                self.calculate_mandelbrot_set(
                    ctx,
                    Restriction::default(),
                    CalculationAction::Reset,
                );
            }
        });
    }

    fn show_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let current_texture = &self.texture_handle;
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
                let current_restriction = self
                    .codomain_history
                    .last()
                    .map(|d| d.restriction())
                    .cloned()
                    .unwrap_or(Restriction::default());

                let map_to_complex = |pos: egui::Pos2| -> Complex<f64> {
                    let min = current_restriction.min();
                    let x = min.re
                        + (pos.x - img_resp.rect.min.x) as f64 / img_resp.rect.width() as f64
                            * current_restriction.width();
                    let y = min.im
                        + (pos.y - img_resp.rect.min.y) as f64 / img_resp.rect.height() as f64
                            * current_restriction.height();
                    Complex::new(x, y)
                };

                let new_restriction = Restriction::from_two_points(
                    map_to_complex(pos_start),
                    map_to_complex(pos_end),
                    self.resolution_x,
                    self.resolution_y,
                );

                self.calculate_mandelbrot_set(ctx, new_restriction, CalculationAction::PushNewZoom);
            }
        });
    }
}

impl eframe::App for MandelbrotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.calculation_receiver
            && let Ok(result) = rx.try_recv()
        {
            self.handle_calculation_result(result);
        }
        self.show_left_sidepanel(ctx);
        self.show_central_panel(ctx);
    }
}
