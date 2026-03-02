use eframe::egui;
use mandelbrot_test::MandelbrotApp;

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
