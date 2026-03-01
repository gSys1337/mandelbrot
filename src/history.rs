use eframe::egui::{Rect, TextureHandle};

pub struct Domain {
    pub rect: Rect,
    pub gray_image: Vec<f64>,
    pub texture: TextureHandle,
}
