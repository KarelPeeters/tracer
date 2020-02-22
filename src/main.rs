use std::time::Instant;

use image::ImageBuffer;
use nalgebra::{Rotation3, Unit, Vector3};
use palette::{Alpha, LinSrgba, named, Srgb, Srgba};
use rand::{Rng, thread_rng};
use rand::distributions::{Bernoulli, Distribution, Uniform};
use rand_distr::UnitSphere;

use crate::geometry::{Hit, Plane, Ray, reflect, Shape, Sphere};
use crate::material::Material;

type Vec3 = Vector3<f32>;
type Point3 = nalgebra::Point3<f32>;
type Color = palette::LinSrgb;

mod geometry;
mod material;

#[derive(Debug)]
struct Object {
    shape: Shape,
    material: Material,
}

#[derive(Debug)]
struct Scene {
    shapes: Vec<Object>,
    sky: Color,
}

impl Scene {
    fn first_hit(&self, ray: &Ray) -> Option<(&Object, Hit)> {
        self.shapes.iter()
            .flat_map(|s| s.shape.intersect(ray).map(|h| (s, h)))
            .min_by(|(_, ah), (_, bh)| ah.t.partial_cmp(&bh.t).unwrap())
    }

    fn trace<R: Rng>(&self, ray: &Ray, rand: &mut R, depth_left: usize) -> Option<Color> {
        if depth_left == 0 { return None; }

        if let Some((object, hit)) = self.first_hit(ray) {
            match object.material {
                Material::Fixed { color } => Some(color),
                Material::Mixed { color, diff_prob } => {
                    let diff = diff_prob.sample(rand);

                    let next_direction = if diff {
                        //diffuse, pick random direction away from surface
                        let [x, y, z] = UnitSphere.sample(rand);
                        let next_direction = Vec3::new(x, y, z);
                        //flip if into surface
                        next_direction.scale(next_direction.dot(&hit.normal).signum());
                        Unit::new_unchecked(next_direction)
                    } else {
                        //mirror
                        reflect(&ray.direction, &hit.normal)
                    };

                    let mut next_ray = Ray {
                        start: hit.point,
                        direction: next_direction,
                    };
                    next_ray.start = next_ray.at(0.01);

                    self.trace(&next_ray, rand, depth_left - 1)
                        .map(|c| c * color)
                }
            }
        } else {
            Some(self.sky)
        }
    }
}

#[derive(Debug)]
struct Camera {
    position: Point3,
    direction: Unit<Vec3>,
    fov_vertical: f32,
    fov_horizontal: f32,
}

impl Camera {
    //TODO fix distortion on the top and bottom of the image and
    //     this also doesn't work for near-vertical camera directions yet
    fn ray(&self, width: f32, height: f32, xi: f32, yi: f32) -> Ray {
        let pitch = self.fov_vertical * (yi / height - 0.5);
        let yaw = self.fov_horizontal * (xi / width - 0.5);
        let rot = Rotation3::from_euler_angles(
            pitch,
            yaw,
            0.0,
        );

        Ray {
            start: self.position.clone(),
            direction: rot * &self.direction,
        }
    }
}

fn main() {
    let scene = Scene {
        sky: Srgb::from_format(named::BLACK).into_linear(),
        shapes: vec![
            Object {
                shape: Shape::Sphere(Sphere { center: Point3::new(0.0, 0.0, 5.0), radius: 1.0 }),
                material: Material::Mixed { color: Srgb::from_format(named::PINK).into_linear(), diff_prob: Bernoulli::new(0.1).unwrap() },
            },
            Object {
                shape: Shape::Sphere(Sphere { center: Point3::new(-3.0, 0.0, 5.0), radius: 1.0 }),
                material: Material::Mixed { color: Srgb::from_format(named::GREEN).into_linear(), diff_prob: Bernoulli::new(0.1).unwrap() },
            },
            Object {
                shape: Shape::Sphere(Sphere { center: Point3::new(3.0, 0.0, 5.0), radius: 1.0 }),
                material: Material::Mixed { color: Srgb::from_format(named::RED).into_linear(), diff_prob: Bernoulli::new(0.1).unwrap() },
            },
            Object {
                shape: Shape::Plane(Plane { point: Point3::new(0.0, -1.0, 0.0), normal: Vec3::y_axis() }),
                material: Material::Mixed { color: Srgb::from_format(named::DARKGRAY).into_linear(), diff_prob: Bernoulli::new(1.0).unwrap() },
            },
            Object {
                shape: Shape::Sphere(Sphere { center: Point3::new(10_000.0, 20_000.0, 4_000.0), radius: 1000.0 }),
                material: Material::Fixed { color: Srgb::from_format(named::WHITE).into_linear() * 400f32 },
            }
        ],
    };

    let width = 1920 / 8;
    let height = 1080 / 8;
    let fov: f32 = 90.0;

    let camera = Camera {
        position: Point3::new(0.0, 0.5, 0.0),
        direction: Unit::new_normalize(Vec3::new(0.0, 0.0, 1.0)),
        fov_vertical: fov.to_radians() * height as f32 / width as f32,
        fov_horizontal: fov.to_radians(),
    };

    let max_depth = 5;
    let sample_count = 100 * 100 * 10;

    let mut rand = thread_rng();
    let mut prev_y = std::u32::MAX;

    let start = Instant::now();

    let image: ImageBuffer<image::Rgba<u8>, _> = ImageBuffer::from_fn(width, height, |x, y| {
        if prev_y != y {
            prev_y = y;
            println!("y={}", y)
        }

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
        let data = Srgba::from_linear(average).into_format();
        image::Rgba([data.red, data.green, data.blue, data.alpha])
    });

    image.save("ignored/output.png").expect("Failed to save image");

    let end = Instant::now();
    println!("Render took {}s", (end - start).as_secs_f32());

    println!("Hello, world!");
}

