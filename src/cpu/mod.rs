use image::ImageBuffer;
use nalgebra::Unit;
use rand::{Rng, thread_rng};
use rand::distributions::Distribution;
use rand_distr::UnitSphere;
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::common::Renderer;
use crate::common::scene::{Camera, Color, Object, Point3, Scene, Transform, Vec3};
use crate::cpu::geometry::{Hit, Intersect, Ray};

mod geometry;

pub struct CpuRenderer {
    pub sample_count: usize,
    pub max_bounces: usize,
    pub anti_alias: bool,
}

impl Renderer for CpuRenderer {
    fn render(&self, scene: &Scene, target: &mut ImageBuffer<image::Rgb<u8>, Vec<u8>>) {
        let camera =
            RayCamera::new(&scene.camera, self.anti_alias, target.width(), target.height());

        target.enumerate_rows_mut().par_bridge().for_each(|(y, row)| {
            println!("y={}", y);
            let mut rng = thread_rng();

            for (x, y, p) in row {
                let mut total = Color::new(0.0, 0.0, 0.0);

                for _ in 0..self.sample_count {
                    let ray = camera.ray(&mut rng, x, y);
                    total += trace_ray(scene, &ray, &mut rng, self.max_bounces);
                }

                let average = total / (self.sample_count as f32);
                let data = palette::Srgb::from_linear(average).into_format();
                *p = image::Rgb([data.red, data.green, data.blue]);
            }
        });
    }
}

struct RayCamera {
    x_span: f32,
    y_span: f32,
    width: f32,
    height: f32,
    transform: Transform,
    anti_alias: bool,
}

impl RayCamera {
    fn new(camera: &Camera, anti_alias: bool, width: u32, height: u32) -> RayCamera {
        let x_span = 2.0 * (camera.fov_horizontal / 2.0).tan();
        RayCamera {
            x_span,
            y_span: x_span * (height as f32) / (width as f32),
            width: width as f32,
            height: height as f32,
            transform: camera.transform.clone(),
            anti_alias,
        }
    }

    fn ray<R: Rng>(&self, rng: &mut R, x: u32, y: u32) -> Ray {
        let (dx, dy) = if self.anti_alias {
            rng.gen()
        } else {
            (0.5, 0.5)
        };

        let x = ((x as f32 + dx) / self.width - 0.5) * self.x_span;
        let y = ((self.height - (y as f32 + dy)) / self.height - 0.5) * self.y_span;

        &self.transform * &Ray {
            start: Point3::origin(),
            direction: Unit::new_normalize(Vec3::new(x, y, -1.0)),
        }
    }
}

const SHADOW_BIAS: f32 = 0.0001;

fn trace_ray<R: Rng>(scene: &Scene, ray: &Ray, rng: &mut R, bounces_left: usize) -> Color {
    if bounces_left == 0 {
        return Color::new(0.0, 0.0, 0.0);
    }

    if let Some((object, hit)) = first_hit(scene, ray) {
        let into = hit.normal.dot(&ray.direction) > 0.0;
        let normal = if into { -hit.normal } else { hit.normal };

        let (weight, next_direction) = if object.material.diffuse {
            let next_direction = Unit::new_unchecked(Vec3::from_column_slice(&UnitSphere.sample(rng)));

            let next_direction = if next_direction.dot(&normal) < 0.0 { -next_direction } else { next_direction };
            let weight = next_direction.dot(&normal);
            (weight, next_direction)
        } else {
            (1.0, reflect_direction(&ray.direction, &normal))
        };

        let next_ray = Ray {
            start: hit.point + SHADOW_BIAS * &*normal,
            direction: next_direction,
        };

        object.material.emission
            + object.material.albedo * weight * trace_ray(scene, &next_ray, rng, bounces_left - 1)
    } else {
        scene.sky_emission
    }
}

fn first_hit<'a>(scene: &'a Scene, ray: &Ray) -> Option<(&'a Object, Hit)> {
    scene
        .objects
        .iter()
        .filter_map(|o| o.intersect(ray).map(|h| (o, h)))
        .min_by(|(_, ah), (_, bh)| ah.t.partial_cmp(&bh.t).expect("t == NaN"))
}

pub fn reflect_direction(vec: &Unit<Vec3>, normal: &Unit<Vec3>) -> Unit<Vec3> {
    Unit::new_unchecked(vec.as_ref() - &normal.scale(2.0 * vec.dot(normal)))
}
