use std::time::Instant;

use image::ImageBuffer;
use nalgebra::Unit;
use palette::{Alpha, LinSrgba, named, Srgb, Srgba};
use rand::thread_rng;
use rand::distributions::{Distribution, Uniform};
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::geometry::{Plane, Shape, Sphere};
use crate::material::Material;
use crate::tracer::{Camera, Light, Object, Point3, Scene, Vec3};

mod geometry;
mod material;
mod tracer;

fn main() {
    let scene = Scene {
        sky: Srgb::from_format(named::BLACK).into_linear(),
        objects: vec![
            Object {
                shape: Shape::Sphere(Sphere { center: Point3::new(0.0, 0.0, 5.0), radius: 1.0 }),
                material: Material::basic(Srgb::from_format(named::PINK).into_linear(), 0.9),
            },
            Object {
                shape: Shape::Sphere(Sphere { center: Point3::new(-3.0, 0.0, 5.0), radius: 1.0 }),
                material: Material::basic(Srgb::from_format(named::GREEN).into_linear(), 0.9),
            },
            Object {
                shape: Shape::Sphere(Sphere { center: Point3::new(3.0, 0.0, 5.0), radius: 1.0 }),
                material: Material::basic(Srgb::from_format(named::RED).into_linear(), 0.9),
            },
            Object {
                shape: Shape::Plane(Plane { point: Point3::new(0.0, -1.0, 0.0), normal: Vec3::y_axis() }),
                material: Material::basic(Srgb::from_format(named::WHITE).into_linear(), 1.0),
            },
        ],
        lights: vec![
            Light {
                position: Point3::new(100.0, 200.0, 40.0),
                color: Srgb::from_format(named::WHITE).into_linear() * 15000f32,
            },
        ],
    };

    let width = 1920;
    let height = 1080;
    let fov: f32 = 90.0;

    let camera = Camera {
        position: Point3::new(0.0, 0.5, 0.0),
        direction: Unit::new_normalize(Vec3::new(0.0, 0.0, 1.0)),
        fov_vertical: fov.to_radians() * height as f32 / width as f32,
        fov_horizontal: fov.to_radians(),
    };

    let max_depth = 7;
    let sample_count = 100;

    let start = Instant::now();

    let mut image: ImageBuffer<image::Rgba<u8>, _> = ImageBuffer::new(width, height);

    image.enumerate_rows_mut().par_bridge().for_each(|(y, row)| {
        println!("y={}", y);
        let mut rand = thread_rng();

        for (x, y, p) in row {
            let mut total = LinSrgba::new(0.0, 0.0, 0.0, 0.0);
            let mut found_count = 0;

            for _ in 0..sample_count {
                let dx = Uniform::from(-0.5..0.5).sample(&mut rand);
                let dy = Uniform::from(-0.5..0.5).sample(&mut rand);

                let ray = camera.ray(
                    width as f32, height as f32,
                    x as f32 + dx, y as f32 + dy,
                );

                let color: LinSrgba = scene.trace(&ray, &mut rand, max_depth)
                    .map(|c| {
                        found_count += 1;
                        Alpha { color: c, alpha: 1.0 }
                    })
                    .unwrap_or(Alpha { color: scene.sky, alpha: 1.0 });

                total += color;
            }

            let average = total / found_count as f32;
            // println!("Average: {:?}, count: {}", average, found_count);
            let data = Srgba::from_linear(average).into_format();
            *p = image::Rgba([data.red, data.green, data.blue, data.alpha]);
        }
    });

    image.save("ignored/output.png").expect("Failed to save image");

    let end = Instant::now();
    println!("Render took {}s", (end - start).as_secs_f32());

    println!("Hello, world!");
}

