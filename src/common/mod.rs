use crate::common::scene::Scene;
use image::ImageBuffer;

pub mod scene;

pub trait Renderer {
    fn render(&self, scene: &Scene, target: &mut ImageBuffer<image::Rgb<u8>, Vec<u8>>);
}
