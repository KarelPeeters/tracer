use std::f32;

use nalgebra::Unit;
use rand::{Rng, thread_rng};
use rand::distributions::Distribution;
use rand_distr::UnitDisc;
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::common::Renderer;
use crate::common::scene::{Camera, Color, Material, Object, Point3, Scene, Transform, Vec2, Vec3, Medium, MaterialType};
use crate::cpu::geometry::{Hit, Intersect, Ray};
use imgref::{ImgRefMut};

pub struct CpuRenderer {
    pub sample_count: usize,
    pub max_bounces: usize,
    pub anti_alias: bool,
}

impl Renderer for CpuRenderer {
    fn render(&self, scene: &Scene, mut target: ImgRefMut<Color>) {
        let camera =
            RayCamera::new(&scene.camera, self.anti_alias, target.width(), target.height());

        target.rows_mut().enumerate().par_bridge().for_each(|(y,row)| {
            println!("y={}", y);
            let mut rng = thread_rng();

            row.iter_mut().enumerate().for_each(|(x, p)| {
                let mut total = Color::new(0.0, 0.0, 0.0);

                for _ in 0..self.sample_count {
                    let ray = camera.ray(&mut rng, x, y);
                    total += trace_ray(scene, &ray, &mut rng, self.max_bounces, true, scene.camera.medium);
                }

                let average = total / (self.sample_count as f32);
                *p = average;
            });
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
    fn new(camera: &Camera, anti_alias: bool, width: usize, height: usize) -> RayCamera {
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

    fn ray<R: Rng>(&self, rng: &mut R, x: usize, y: usize) -> Ray {
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

fn sample_lights<R: Rng>(scene: &Scene, next_start: &Point3, medium: Medium, rng: &mut R, hit: &Hit) -> Color {
    let mut result = Color::new(0.0, 0.0, 0.0);

    for light in &scene.objects {
        if is_black(light.material.emission) { continue; }

        let (weight, target) = light.sample(rng);
        let light_ray = Ray { start: next_start.clone(), direction: Unit::new_normalize(target - next_start) };

        match first_hit(scene, &light_ray) {
            Some((object, light_hit)) if std::ptr::eq(object, light) => {
                let abs_cos = light_ray.direction.dot(&hit.normal).abs();
                let volumetric_mask = color_exp(medium.volumetric_color, light_hit.t);

                result += light.material.emission * weight * abs_cos * volumetric_mask * light.area_seen_from(next_start);
            }
            Some(_) => {} //another object is blocking the light
            None => {} //hit nothing
        }
    }

    result
}

fn color_exp(base: Color, exp: f32) -> Color {
    Color::new(base.red.powf(exp), base.green.powf(exp), base.blue.powf(exp))
}

fn trace_ray<R: Rng>(scene: &Scene, ray: &Ray, rng: &mut R, bounces_left: usize, spectral: bool, medium: Medium) -> Color {
    if bounces_left == 0 {
        return Color::new(0.0, 0.0, 0.0);
    }

    let (t, result) = if let Some((object, mut hit)) = first_hit(scene, ray) {
        if object.material.material_type == MaterialType::Fixed {
            return object.material.albedo;
        }

        let into = hit.normal.dot(&ray.direction) < 0.0;
        let next_medium = if into {
            debug_assert_eq!(medium, object.material.outside);
            object.material.inside
        } else {
            hit.normal = -hit.normal;
            debug_assert_eq!(medium, object.material.inside);
            object.material.outside
        };

        let mut result = Color::new(0.0, 0.0, 0.0);

        if spectral {
            result += object.material.emission;
        }

        let refract_ratio = medium.index_of_refraction / next_medium.index_of_refraction;
        let (weight, trans, next_spectral,  next_direction) = sample_direction(&ray, &hit, &object.material, refract_ratio, rng);
        if !next_spectral {
            let light_start = &hit.point + hit.normal.as_ref() * SHADOW_BIAS;
            let light_contribution = sample_lights(scene, &light_start, medium, rng, &hit);
            result += object.material.albedo * light_contribution;
        }

        let next_ray = Ray {
            start: &hit.point + *next_direction * SHADOW_BIAS,
            direction: next_direction,
        };
        let next_medium = if trans { next_medium } else { medium };
        let next_contribution = trace_ray(scene, &next_ray, rng, bounces_left - 1, next_spectral, next_medium);

        result += object.material.albedo * next_contribution * weight;

        (hit.t, result)
    } else {
        (f32::INFINITY, scene.sky_emission)
    };

    color_exp(medium.volumetric_color, t) * result
}

fn sample_direction<R: Rng>(ray: &Ray, hit: &Hit, material: &Material, refract_ratio: f32, rng: &mut R) -> (f32, bool, bool, Unit<Vec3>) {
    match material.material_type {
        MaterialType::Fixed => panic!("Can't sample direction for MaterialType::Fixed"),
        MaterialType::Diffuse => {
            let disk = Vec2::from_column_slice(&UnitDisc.sample(rng));
            (0.5, false, false, disk_to_hemisphere(&disk, &hit.normal))
        },
        MaterialType::Mirror => {
            (1.0, false, true, reflect_direction(&ray.direction, &hit.normal))
        },
        MaterialType::Transparent => {
            let (trans, dir) = snells_law(&ray.direction, &hit.normal, refract_ratio);
            (1.0, trans, true, dir)
        },
    }
}

fn disk_to_hemisphere(disk: &Vec2, normal: &Unit<Vec3>) -> Unit<Vec3> {
    let z = (1.0 - disk.norm_squared()).sqrt();
    let x_axis = Unit::new_normalize(Vec3::new(-normal.y, normal.x, 0.0));
    let y_axis = Unit::new_unchecked(normal.cross(&x_axis));
    Unit::new_unchecked(*x_axis * disk.x + *y_axis * disk.y + **normal * z)
}

fn reflect_direction(vec: &Unit<Vec3>, normal: &Unit<Vec3>) -> Unit<Vec3> {
    Unit::new_unchecked(vec.as_ref() - &normal.scale(2.0 * vec.dot(normal)))
}

fn snells_law(vec: &Unit<Vec3>, normal: &Unit<Vec3>, r: f32) -> (bool, Unit<Vec3>) {
    let c = normal.dot(vec);
    let x = 1.0 - r*r*(1.0 - c*c);

    if x > 0.0 {
        //actual transparency
        (true, Unit::new_unchecked(**vec * r + **normal * (r *c - x.sqrt())))
    } else {
        //total internal reflection
        (false, reflect_direction(vec, normal))
    }
}

fn first_hit<'a>(scene: &'a Scene, ray: &Ray) -> Option<(&'a Object, Hit)> {
    scene
        .objects
        .iter()
        .filter_map(|o| o.intersect(ray).map(|h| (o, h)))
        .min_by(|(_, ah), (_, bh)| ah.t.partial_cmp(&bh.t).expect("t == NaN"))
}

fn is_black(color: Color) -> bool {
    return color == Color::new(0.0, 0.0, 0.0);
}