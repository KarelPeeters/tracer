#![allow(dead_code)]

use std::fs::read_to_string;
use std::time::Instant;

use nalgebra::Unit;
use palette::{named, Srgb};
use wavefront_obj::obj::{Primitive, Vertex};
use wavefront_obj::obj;

use crate::camera::*;
use crate::geometry::*;
use crate::material::*;
use crate::tracer::*;

mod geometry;
mod material;
mod camera;
mod tracer;

fn colored_spheres() -> Vec<Object> {
    vec![
        Object {
            shape: Shape::Sphere(Sphere {
                center: Point3::new(0.0, 0.0, 5.0),
                radius: 1.0,
            }),
            material: Material::basic(Srgb::from_format(named::PINK).into_linear(), 0.0, 0.0),
        },
        Object {
            shape: Shape::Sphere(Sphere {
                center: Point3::new(-3.0, 0.0, 5.0),
                radius: 1.0,
            }),
            material: Material::basic(Srgb::from_format(named::RED).into_linear(), 0.9, 0.0),
        },
        Object {
            shape: Shape::Sphere(Sphere {
                center: Point3::new(3.0, 0.0, 5.0),
                radius: 1.0,
            }),
            material: Material::basic(Srgb::from_format(named::RED).into_linear(), 0.9, 0.0),
        }
    ]
}

fn plane() -> Vec<Object> {
    vec![
        Object {
            shape: Shape::Plane(Plane {
                point: Point3::new(0.0, -1.0, 0.0),
                normal: Vec3::y_axis(),
            }),
            material: Material::basic(Srgb::from_format(named::GREY).into_linear(), 0.8, 0.0),
        }
    ]
}

fn unit_sphere() -> Vec<Object> {
    vec![
        Object {
            shape: Shape::Sphere(Sphere { center: Point3::new(0.0, 0.0, 0.0), radius: 1.0 }),
            material: Material::basic(Srgb::from_format(named::YELLOW).into_linear(), 1.0, 0.0),
        }
    ]
}

fn vertex_to_point(vertex: &Vertex) -> Point3 {
    Point3::new(vertex.x as f32, vertex.y as f32, vertex.z as f32)
}

fn obj_to_objects(obj: &obj::Object) -> Vec<Object> {
    let mut result = Vec::new();

    for geometry in &obj.geometry {
        for shape in &geometry.shapes {
            match shape.primitive {
                Primitive::Point(_) => {}
                Primitive::Line(_, _) => {}
                Primitive::Triangle((avi, ..), (bvi, ..), (cvi,..)) => {
                    let a = vertex_to_point(&obj.vertices[avi]);
                    let b = vertex_to_point(&obj.vertices[bvi]);
                    let c = vertex_to_point(&obj.vertices[cvi]);

                    let triangle = Object {
                        shape: Shape::Triangle(Triangle::new(a, b, c)),
                        material: Material::basic(Srgb::from_format(named::WHITE).into_linear(), 1.0, 0.0),
                    };

                    result.push(triangle)
                }
            }
        }
    }

    result
}

fn main() {
    let mut scene = Scene {
        sky: Color::new(0.02, 0.02, 0.02),
        objects: Vec::new(),
        lights: vec![
            Light {
                position: Point3::new(100.0, 200.0, -40.0),
                color: Srgb::from_format(named::WHITE).into_linear() * 15000f32,
            }
        ],
    };

    scene.objects.extend(colored_spheres());
    scene.objects.extend(plane());
    // scene.objects.extend(unit_sphere());

    let str = read_to_string("models/monkey.obj").expect("Error while reading model");
    let obj_set = wavefront_obj::obj::parse(&str).expect("Error while parsing model");
    let obj = obj_set.objects.first().expect("No object found");

    // scene.objects.extend(obj_to_objects(obj));

    let scene = scene;

    println!("scene: {:?}", scene);

    let width = 1920 / 8;
    let height = 1080 / 8;

    let fov = 90.0f32.to_radians();

    let max_depth = 7;
    let sample_count = 100;

    let cam_target = Point3::new(0.0, 0.0, 0.0);
    let cam_position = Point3::new(0.0, 0.0, -3.0);
    let cam_direction = Unit::new_normalize(&cam_target - &cam_position);

    // let camera = OrthographicCamera { position: cam_position, direction: cam_direction, width: 2.0, };

    let camera = PerspectiveCamera {
        position: cam_position,
        direction: cam_direction,
        fov_vertical: fov * height as f32 / width as f32,
        fov_horizontal: fov,
    };

    let start = Instant::now();

    let image = trace_image(&scene, &camera, width, height, max_depth, sample_count);

    image.save("ignored/output.png").expect("Failed to save image");

    let end = Instant::now();
    println!("Render took {}s", (end - start).as_secs_f32());

    println!("Hello, world!");
}