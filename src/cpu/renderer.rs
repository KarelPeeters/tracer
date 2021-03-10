use std::f32;
use std::sync::atomic::{AtomicUsize, Ordering};

use imgref::ImgRefMut;
use rand::{Rng, thread_rng};
use rand::distributions::Distribution;
use rand::seq::SliceRandom;
use rand_distr::UnitDisc;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use rayon::ThreadPoolBuilder;

use crate::common::math::{Norm, Point3, Transform, Unit, Vec2, Vec3};
use crate::common::Renderer;
use crate::common::scene::{Camera, Color, MaterialType, Medium, Object, Scene};
use crate::cpu::geometry::{Hit, Intersect, Ray};

pub struct CpuRenderer {
    pub sample_count: usize,
    pub max_bounces: usize,
    pub anti_alias: bool,
}

impl Renderer for CpuRenderer {
    fn render(&self, scene: &Scene, mut target: ImgRefMut<Color>) {
        let camera =
            RayCamera::new(&scene.camera, self.anti_alias, target.width(), target.height());

        ThreadPoolBuilder::new().num_threads(8).build_global().expect("Failed to build global thread pool");

        let progress_rows_done = AtomicUsize::default();
        let height = target.height();
        let progress_div = if height > 1000 { height / 1000 } else { 1 };

        let mut rows: Vec<_> = target.rows_mut().enumerate().collect();
        rows.shuffle(&mut thread_rng());

        rows.par_iter_mut().for_each_init(|| thread_rng(), |rng, (y, row)| {
            let progress = progress_rows_done.fetch_add(1, Ordering::Relaxed);

            if progress % progress_div == 0 {
                println!("Progress {:.3}", progress as f32 / height as f32)
            }

            row.iter_mut().enumerate().for_each(|(x, p)| {
                let mut total = Color::new(0.0, 0.0, 0.0);

                for _ in 0..self.sample_count {
                    let ray = camera.ray(rng, x, *y);
                    total += trace_ray(scene, &ray, rng, self.max_bounces, true, scene.camera.medium);
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
        let x_span = 2.0 * (camera.fov_horizontal.radians / 2.0).tan();
        RayCamera {
            x_span,
            y_span: x_span * (height as f32) / (width as f32),
            width: width as f32,
            height: height as f32,
            transform: camera.transform,
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

        self.transform * &Ray {
            start: Point3::origin(),
            direction: Vec3::new(x, y, -1.0).normalized(),
        }
    }
}

const SHADOW_BIAS: f32 = 0.0001;

fn sample_lights<R: Rng>(scene: &Scene, next_start: Point3, medium: Medium, rng: &mut R, hit: &Hit) -> Color {
    let mut result = Color::new(0.0, 0.0, 0.0);

    for light in &scene.objects {
        if is_black(light.material.emission) { continue; }

        let (weight, target) = light.sample(rng);
        let light_ray = Ray { start: next_start, direction: (target - next_start).normalized() };

        match first_hit(scene, &light_ray) {
            Some((object, light_hit)) if std::ptr::eq(object, light) => {
                let abs_cos = light_ray.direction.dot(*hit.normal).abs();
                let volumetric_mask = color_exp(medium.volumetric_color, light_hit.t);

                result += light.material.emission * weight * abs_cos * volumetric_mask * light.area_seen_from(next_start);
            }
            Some(_) => {} //another object is blocking the light
            None => {} //hit nothing
        }
    }

    result
}

fn trace_ray<R: Rng>(scene: &Scene, ray: &Ray, rng: &mut R, bounces_left: usize, spectral: bool, medium: Medium) -> Color {
    if bounces_left == 0 {
        return Color::new(0.0, 0.0, 0.0);
    }

    let (t, result) = if let Some((object, mut hit)) = first_hit(scene, ray) {
        if let MaterialType::Fixed = object.material.material_type {
            return object.material.albedo;
        }

        let into = hit.normal.dot(*ray.direction) < 0.0;
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
        let (weight, trans, next_spectral, next_direction) = sample_direction(&ray, &hit, object.material.material_type, refract_ratio, rng);
        if !next_spectral {
            let light_start = hit.point + (*hit.normal * SHADOW_BIAS);
            let light_contribution = sample_lights(scene, light_start, medium, rng, &hit);
            result += object.material.albedo * light_contribution;
        }

        let next_ray = Ray {
            start: hit.point + (*next_direction * SHADOW_BIAS),
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

fn sample_direction<R: Rng>(ray: &Ray, hit: &Hit, material_type: MaterialType, refract_ratio: f32, rng: &mut R) -> (f32, bool, bool, Unit<Vec3>) {
    match material_type {
        MaterialType::Fixed => panic!("Can't sample direction for MaterialType::Fixed"),
        MaterialType::Diffuse => {
            let disk = Vec2::from_slice(&UnitDisc.sample(rng));
            (0.5, false, false, disk_to_hemisphere(disk, hit.normal))
        }
        MaterialType::Mirror => {
            (1.0, false, true, reflect_direction(ray.direction, hit.normal))
        }
        MaterialType::Transparent => {
            let (trans, dir) = snells_law(ray.direction, hit.normal, refract_ratio);
            (1.0, trans, true, dir)
        }
        MaterialType::DiffuseMirror(f) => {
            if rng.gen::<f32>() < f {
                sample_direction(ray, hit, MaterialType::Diffuse, refract_ratio, rng)
            } else {
                sample_direction(ray, hit, MaterialType::Mirror, refract_ratio, rng)
            }
        }
    }
}

fn disk_to_hemisphere(disk: Vec2, normal: Unit<Vec3>) -> Unit<Vec3> {
    let z = (1.0 - disk.norm_squared()).sqrt();
    let x_axis = Vec3::new(-normal.y, normal.x, 0.0).normalized();
    let y_axis = Unit::new_unchecked(normal.cross(*x_axis));
    Unit::new_unchecked((*x_axis * disk.x) + (*y_axis * disk.y) + (*normal * z))
}

fn reflect_direction(vec: Unit<Vec3>, normal: Unit<Vec3>) -> Unit<Vec3> {
    Unit::new_unchecked((*vec) - (*normal * (2.0 * vec.dot(*normal))))
}

/// Compute the outgoing direction according to Snell's law
/// (https://en.wikipedia.org/wiki/Snell%27s_law#Vector_form), including total internal reflection.
/// `vec` and normal should point in opposite directions
fn snells_law(vec: Unit<Vec3>, normal: Unit<Vec3>, r: f32) -> (bool, Unit<Vec3>) {
    let c = -normal.dot(*vec);
    let x = 1.0 - r * r * (1.0 - c * c);
    debug_assert!(c >= 0.0, "vec and normal should point in opposite directions");

    if x > 0.0 {
        //actual transparency
        (true, Unit::new_unchecked((*vec * r) + (*normal * (r * c - x.sqrt()))))
    } else {
        //total internal reflection
        (false, reflect_direction(vec, normal))
    }
}

fn first_hit<'a>(scene: &'a Scene, ray: &Ray) -> Option<(&'a Object, Hit)> {
    //this function used to be implemented with iterators but that was a bit slower
    let mut closest_hit = Hit {
        t: f32::INFINITY,
        point: Default::default(),
        normal: Vec3::z_axis(),
    };
    let mut closest_object = scene.objects.first()?;

    for object in &scene.objects {
        if let Some(hit) = object.intersect(ray) {
            if hit.t < closest_hit.t {
                closest_hit = hit;
                closest_object = object;
            }
        }
    }

    if closest_hit.t == f32::INFINITY {
        None
    } else {
        Some((closest_object, closest_hit))
    }
}

fn is_black(color: Color) -> bool {
    return color == Color::new(0.0, 0.0, 0.0);
}

fn color_exp(base: Color, exp: f32) -> Color {
    Color::new(fast_powf(base.red, exp), fast_powf(base.green, exp), fast_powf(base.blue, exp))
}

fn fast_powf(base: f32, exp: f32) -> f32 {
    debug_assert!(base >= 0.0);
    debug_assert!(!(base == 0.0 && exp == 0.0));

    if base == 0.0 || base == 1.0 || exp == 1.0 {
        base
    } else if exp.is_infinite() {
        if (base > 1.0) ^ (exp < 0.0) {
            f32::INFINITY
        } else {
            0.0
        }
    } else {
        base.powf(exp)
    }
}
