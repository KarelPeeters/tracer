use image::ImageBuffer;
use nalgebra::{Rotation3, Unit, Vector3};
use palette::{named, Srgb, Shade};

use crate::geometry::{Hit, Plane, Ray, Shape, Sphere};
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

    fn trace(&self, ray: &Ray, depth_left: usize) -> Option<Color> {
        if depth_left == 0 { return None; }

        if let Some((object, hit)) = self.first_hit(ray) {
            match object.material {
                Material::Fixed(color) => Some(color),
                Material::Mirror => {
                    let reflect_direction = Unit::new_normalize(ray.direction.as_ref() -
                        &hit.normal.scale(2.0 * ray.direction.dot(&hit.normal)));
                    let mut reflect_ray = Ray {
                        start: hit.point,
                        direction: reflect_direction,
                    };
                    //move a bit ahead so we don't collide with the same object again
                    reflect_ray.start = reflect_ray.at(0.01);

                    let color = self.trace(&reflect_ray, depth_left - 1);

                    color.map(|c| c.darken(0.05))
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
    fn ray(&self, width: u32, height: u32, xi: u32, yi: u32) -> Ray {
        let pitch = self.fov_vertical * ((yi as f32) / (height as f32) - 0.5);
        let yaw = self.fov_horizontal * ((xi as f32) / (width as f32) - 0.5);
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
        sky: Srgb::from_format(named::DEEPSKYBLUE).into_linear(),
        shapes: vec![
            Object {
                shape: Shape::Sphere(Sphere { center: Point3::new(0.0, 0.0, 5.0), radius: 1.0 }),
                material: Material::Mirror,
            },
            Object {
                shape: Shape::Sphere(Sphere { center: Point3::new(-3.0, 0.0, 5.0), radius: 1.0 }),
                material: Material::Fixed(Srgb::from_format(named::GREEN).into_linear()),
            },
            Object {
                shape: Shape::Sphere(Sphere { center: Point3::new(3.0, 0.0, 5.0), radius: 1.0 }),
                material: Material::Fixed(Srgb::from_format(named::RED).into_linear()),
            },
            Object {
                shape: Shape::Plane(Plane { point: Point3::new(0.0, -2.0, 0.0), normal: Vec3::y_axis() }),
                material: Material::Fixed(Srgb::from_format(named::GRAY).into_linear()),
            },
        ],
    };

    let camera = Camera {
        position: Point3::new(0.0, 0.1, 0.0),
        direction: Unit::new_normalize(Vec3::new(0.0, 0.0, 1.0)),
        fov_vertical: 90.0f32.to_radians(),
        fov_horizontal: 90.0f32.to_radians(),
    };

    let width = 1000;
    let height = 1000;

    //TODO multiple rays per pixel for antialiasing
    let image: ImageBuffer<image::Rgba<u8>, _> = ImageBuffer::from_fn(width, height, |x, y| {
        let ray = camera.ray(width, height, x, y);
        let color = scene.trace(&ray, 50);

        match color {
            None => image::Rgba([0, 0, 0, 0]),
            Some(color) => {
                let data = Srgb::from_linear(color).into_format();
                image::Rgba([data.red, data.green, data.blue, 255])
            }
        }
    });
    image.save("output.png").unwrap();

    println!("Hello, world!");
}

