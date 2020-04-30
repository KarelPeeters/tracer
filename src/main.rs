#![allow(dead_code)]

use std::f32::consts::PI;
use std::fs::read_to_string;
use std::time::Instant;

use imgref::Img;
use wavefront_obj::obj;

use crate::common::math::{Norm, Point3, Transform, Vec3};
use crate::common::Renderer;
use crate::common::scene::{Camera, Color, Material, MaterialType, Medium, Object, Scene, Shape};
use crate::common::util::{obj_to_triangles, to_image};
use crate::cpu::CpuRenderer;

mod common;
mod cpu;

fn color_by_name(name: &str) -> Color {
    palette::Srgb::from_format(palette::named::from_str(name).expect("Invalid color name"))
        .into_linear()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let black = Color::new(0.0, 0.0, 0.0);
    let white = Color::new(1.0, 1.0, 1.0);

    let vacuum = Medium {
        index_of_refraction: 1.0,
        volumetric_color: white,
    };

    let medium_glass = Medium {
        index_of_refraction: 1.52,
        volumetric_color: Color::new(1.0, 0.1, 0.1),
    };

    let material_floor = Material {
        material_type: MaterialType::Diffuse,

        albedo: color_by_name("gray"),
        emission: black,

        inside: vacuum,
        outside: vacuum,
    };

    let material_glass = Material {
        material_type: MaterialType::Transparent,

        albedo: white,//color_by_name("red"),
        emission: black,

        inside: medium_glass,
        outside: vacuum,
    };

    let material_light = Material {
        material_type: MaterialType::Diffuse,
        albedo: black,
        emission: Color::new(1.0, 1.0, 1.0) * 1000.0,

        inside: vacuum,
        outside: vacuum,
    };

    let mut scene = Scene {
        objects: vec![
            Object {
                shape: Shape::Plane,
                material: material_floor,
                transform: Transform::rotation(Vec3::x_axis(), PI / 2.0),
            },
            Object {
                shape: Shape::Sphere,
                material: material_glass,
                transform: Transform::translation(Vec3::new(0.0, 1.0, 0.0)),
            },
            Object {
                shape: Shape::Sphere,
                material: material_light,
                transform: Transform::translation(Vec3::new(10.0, 10.0, -5.0)),
            }
        ],
        sky_emission: white,
        camera: Camera {
            fov_horizontal: 70f32.to_radians(),
            transform: Transform::look_at(Point3::new(0.0, 1.5, 5.0), Point3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 0.0).normalized()),

            medium: vacuum,
        },
    };

    if false {
        let objects = obj::parse(read_to_string("ignored/models/cube.obj")?).expect("Error while parsing obj file");
        let object = objects.objects.first().expect("No object found");
        scene.objects.extend(obj_to_triangles(object, material_floor, Default::default()).skip(2).take(1));
    }

    let renderer = CpuRenderer {
        sample_count: 100,
        max_bounces: 8,
        anti_alias: true,
    };

    let div = 8;
    let (width, height) = (1920, 1080);
    let (width, height) = if div == 0 { (1, 1) } else { (width / div, height / div) };

    let mut result = Img::new(vec![black; width * height], width, height);

    let start = Instant::now();
    renderer.render(&scene, result.as_mut());
    println!("Render took {:?}s", (Instant::now() - start).as_secs_f32());

    let (result, clipped) = to_image(result.as_ref());
    result.save("ignored/output.png")?;
    clipped.save("ignored/output_clipped.png")?;

    Ok(())
}
