#![allow(dead_code)]

use std::time::Instant;

use image::ImageBuffer;
use nalgebra::{convert, Similarity3};

use crate::common::scene::Vec3;
use crate::common::scene::{Camera, Color, Material, Object, Scene, Shape, Transform};
use crate::common::Renderer;
use crate::cpu::CpuRenderer;
use std::f32::consts::PI;

mod common;
mod cpu;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let width = 1920 / 8;
    let height = 1080 / 8;

    let fov_horizontal: f32 = 100.0;

    let scene = Scene {
        objects: vec![Object {
            shape: Shape::Triangle,
            material: Material {
                color: Color::new(1.0, 0.0, 0.0),
            },
            transform: convert(Similarity3::new(
                Vec3::new(0.0, 0.0, -2.0),
                0.0 * PI / 2.0 * Vec3::x(),
                1.0,
            )),
        }],
        sky: Material {
            color: Color::new(0.0, 0.2, 0.8),
        },
        camera: Camera {
            fov_horizontal: fov_horizontal.to_radians(),
            transform: Transform::identity(),
        },
    };

    let renderer = CpuRenderer {
        sample_count: 10,
        max_bounces: 8,
        anti_alias: true,
    };

    let mut result = ImageBuffer::new(width, height);

    let start = Instant::now();
    renderer.render(&scene, &mut result);
    println!("Render took {:?}s", (Instant::now() - start).as_secs_f32());

    result.save("output.png")?;

    Ok(())
}
