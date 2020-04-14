#![allow(dead_code)]

use std::f32::consts::PI;
use std::time::Instant;

use image::ImageBuffer;
use nalgebra::{convert, Rotation3, Translation3, UnitQuaternion};

use crate::common::Renderer;
use crate::common::scene::{Camera, Color, Material, Object, Point3, Scene, Shape, Transform};
use crate::common::scene::Vec3;
use crate::cpu::CpuRenderer;

mod common;
mod cpu;

fn color_by_name(name: &str) -> Color {
    palette::Srgb::from_format(palette::named::from_str(name).expect("Invalid color name"))
        .into_linear()
}

fn camera_transform(eye: &Point3, target: &Point3, up: &Vec3) -> Transform {
    let translation = Translation3::from(eye.coords);
    let rotation = UnitQuaternion::look_at_rh(&(target - eye), up);
    convert(translation * rotation)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let div = 4;

    let width = 1920 / div;
    let height = 1080 / div;

    let fov_horizontal: f32 = 100.0;

    let vertical = Rotation3::new(Vec3::new(PI / 2.0, 0.0, 0.0));
    let black = Color::new(0.0, 0.0, 0.0);

    let scene = Scene {
        objects: vec![
            Object {
                shape: Shape::Plane,
                material: Material {
                    albedo: color_by_name("gray"),
                    emission: black,
                    diffuse: true,
                },
                transform: convert(Translation3::new(0.0, -1.0, 0.0) * &vertical),
            },
            Object {
                shape: Shape::Cylinder,
                material: Material {
                    albedo: color_by_name("red"),
                    emission: black,
                    diffuse: true,
                },
                transform: convert(Translation3::new(0.0, 0.0, 0.0)),
            },
            Object {
                shape: Shape::Sphere,
                material: Material {
                    albedo: black,
                    emission: Color::new(1.0, 1.0, 1.0) * 1000.0,
                    diffuse: true,
                },
                transform: convert(Translation3::new(10.0, 10.0, -5.0)),
            }
        ],
        sky_emission: color_by_name("darkgray"),
        camera: Camera {
            fov_horizontal: fov_horizontal.to_radians(),
            transform: camera_transform(&Point3::new(0.0, 0.0, 3.0), &Point3::new(0.0, 0.0, 0.0), &Vec3::new(0.0, 1.0, 0.0)),
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
