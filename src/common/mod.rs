use image::ImageBuffer;

use crate::common::scene::Scene;

pub mod scene;

pub trait Renderer {
    fn render(&self, scene: &Scene, target: &mut ImageBuffer<image::Rgb<u8>, Vec<u8>>);
}
