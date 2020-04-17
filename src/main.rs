#![allow(dead_code)]

use std::f32::consts::PI;
use std::time::Instant;

use image::ImageBuffer;
use nalgebra::{convert, Rotation3, Translation3, UnitQuaternion, Similarity3};

use crate::common::Renderer;
use crate::common::scene::{Camera, Color, Material, Object, Point3, Scene, Shape, Transform, Medium, MaterialType};
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
    let rotation = UnitQuaternion::look_at_rh(&(target - eye), up).inverse();
    convert(translation * rotation)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let vertical = Rotation3::new(Vec3::new(PI / 2.0, 0.0, 0.0));
    let black = Color::new(0.0, 0.0, 0.0);
    let white = Color::new(1.0, 1.0, 1.0);

    let vacuum = Medium {
        index_of_refraction: 1.0,
        volumetric_color: white,
    };

    let glass = Medium {
        index_of_refraction: 1.52,
        volumetric_color: Color::new(1.0, 0.1, 0.1),
    };

    let scene = Scene {
        objects: vec![
            Object {
                shape: Shape::Plane,
                material: Material {
                    material_type: MaterialType::Diffuse,

                    albedo: color_by_name("gray"),
                    emission: black,

                    inside: vacuum,
                    outside: vacuum,
                },
                transform: (Translation3::new(0.0, -1.0, 0.0) * &vertical).into(),
            },
            Object {
                shape: Shape::Sphere,
                material: Material {
                    material_type: MaterialType::Transparent,

                    albedo: white,//color_by_name("red"),
                    emission: black,

                    inside: glass,
                    outside: vacuum,
                },
                transform: Translation3::new(0.0, 0.0, 0.0).into(),
            },
            Object {
                shape: Shape::Sphere,
                material: Material {
                    material_type: MaterialType::Diffuse,
                    albedo: black,
                    emission: Color::new(1.0, 1.0, 1.0) * 1000.0,

                    inside: vacuum,
                    outside: vacuum,
                },
                transform: Similarity3::from_parts(Translation3::new(10.0, 10.0, -5.0), UnitQuaternion::identity(), 1.0).into(),
            }
        ],
        sky_emission: white,
        camera: Camera {
            fov_horizontal: 70f32.to_radians(),
            transform: camera_transform(&Point3::new(0.0, 0.3, 5.0), &Point3::new(0.0, 0.0, 0.0), &Vec3::new(0.0, 1.0, 0.0)),

            medium: vacuum,
        },
    };

    let renderer = CpuRenderer {
        sample_count: 100,
        max_bounces: 8,
        anti_alias: true,
    };

    let div = 8;
    let width = 1920 / div;
    let height = 1080 / div;

    let mut result = ImageBuffer::new(width, height);

    let start = Instant::now();
    renderer.render(&scene, &mut result);
    println!("Render took {:?}s", (Instant::now() - start).as_secs_f32());

    result.save("ignored/output.png")?;

    Ok(())
}
