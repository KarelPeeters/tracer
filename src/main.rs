#![allow(dead_code)]

use std::f32::consts::PI;
use std::time::Instant;

use image::ImageBuffer;
use nalgebra::{convert, Rotation3, Translation3};

use crate::common::Renderer;
use crate::common::scene::{Camera, Color, Material, Object, Scene, Shape};
use crate::common::scene::Vec3;
use crate::cpu::CpuRenderer;

mod common;
mod cpu;

fn color_by_name(name: &str) -> Color {
    palette::Srgb::from_format(palette::named::from_str(name).expect("Invalid color name"))
        .into_linear()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let width = 1920 / 8;
    let height = 1080 / 8;

    let fov_horizontal: f32 = 100.0;

    let vertical = Rotation3::new(PI / 2.0 * Vec3::x());
    let black = Color::new(0.0, 0.0, 0.0);

    let scene = Scene {
        objects: vec![
            Object {
                shape: Shape::Plane,
                material: Material {
                    albedo: color_by_name("darkred"),
                    emission: black,
                    diffuse: true,
                },
                transform: convert(Translation3::new(0.0, -1.0, 0.0) * vertical),
            },
            Object {
                shape: Shape::Sphere,
                material: Material {
                    albedo: color_by_name("red"),
                    emission: black,
                    diffuse: true,
                },
                transform: convert(vertical),
            },
        ],
        sky_emission: color_by_name("white") * 1.5,
        camera: Camera {
            fov_horizontal: fov_horizontal.to_radians(),
            transform: convert(Translation3::new(0.0, 0.0, 2.0)),
        },
    };

    let renderer = CpuRenderer {
        sample_count: 100,
        max_bounces: 8,
        anti_alias: true,
    };

    let mut result = ImageBuffer::new(width, height);

    let start = Instant::now();
    renderer.render(&scene, &mut result);
    println!("Render took {:?}s", (Instant::now() - start).as_secs_f32());

    result.save("ignored/output.png")?;

    Ok(())
}
