use std::cmp::{max};
use std::f32;



use rand::{Rng};
use rand::distributions::Distribution;

use rand_distr::UnitDisc;
use rayon::iter::{ParallelIterator};


use crate::common::math::{Norm, Point3, Transform, Unit, Vec2, Vec3};
use crate::common::scene::{Camera, Color, MaterialType, Medium, Object, Scene};
use crate::cpu::geometry::{Hit, Intersect, Ray};
use crate::cpu::stats::ColorVarianceEstimator;

#[derive(Debug)]
pub struct CpuRenderSettings {
    pub stop_condition: StopCondition,
    pub max_bounces: u32,
    pub anti_alias: bool,
    pub strategy: Strategy,
}

#[derive(Debug, Copy, Clone)]
pub enum StopCondition {
    SampleCount(u32),
    Variance { min_samples: u32, max_relative_variance: f32 },
}

#[derive(Debug, Copy, Clone)]
pub enum Strategy {
    Simple,
    SampleLights,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct PixelResult {
    pub color: Color,
    pub variance: Color,
    pub rel_variance: Color,
    pub samples: u32,
}

impl CpuRenderSettings {
    pub fn calculate_pixel(&self, scene: &Scene, camera: &RayCamera, rng: &mut impl Rng, x: u32, y: u32) -> PixelResult {
        let mut estimator = ColorVarianceEstimator::default();

        while !self.stop_condition.is_done(&estimator) {
            let ray = camera.ray(rng, x, y);
            let color = trace_ray(scene, self.strategy, &ray, rng, self.max_bounces, true, scene.camera.medium);
            estimator.update(color);
        }

        let variance = estimator.variance().unwrap_or(Color::new(0.0, 0.0, 0.0));
        PixelResult {
            color: estimator.mean,
            variance,
            rel_variance: variance / (estimator.mean + Color::new(1.0, 1.0, 1.0)),
            samples: estimator.count,
        }
    }
}

impl StopCondition {
    fn is_done(self, estimator: &ColorVarianceEstimator) -> bool {
        fn variance_lte(estimator: &ColorVarianceEstimator, right: f32) -> bool {
            //TODO figure out a better way to allow blackness and add a mechanism to ignore variance in huge means
            let variance = estimator.variance().expect("Not enough samples to even compute the variance!");
            let rel_variance = variance / (estimator.mean + Color::new(1.0, 1.0, 1.0));

            //we care about the variance of the mean, not the variance of the values themselves
            let left = rel_variance / (estimator.count as f32).sqrt();
            left.red <= right && left.green <= right && left.blue <= right
        }

        match self {
            StopCondition::SampleCount(samples) =>
                estimator.count >= samples,
            StopCondition::Variance { min_samples, max_relative_variance } =>
                estimator.count >= max(min_samples, 2) &&
                    variance_lte(estimator, max_relative_variance),
        }
    }
}


pub struct RayCamera {
    x_span: f32,
    y_span: f32,
    width: f32,
    height: f32,
    transform: Transform,
    anti_alias: bool,
}

impl RayCamera {
    pub fn new(camera: &Camera, anti_alias: bool, width: u32, height: u32) -> RayCamera {
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

    fn ray<R: Rng>(&self, rng: &mut R, x: u32, y: u32) -> Ray {
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

    //TODO pre-filter out the lights, this scales badly with the amount of other options
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

fn trace_ray<R: Rng>(
    scene: &Scene,
    strategy: Strategy,
    ray: &Ray,
    rng: &mut R,
    bounces_left: u32,
    specular: bool,
    medium: Medium,
) -> Color {
    if bounces_left == 0 {
        return Color::new(0.0, 0.0, 0.0);
    }

    let (t, result) = if let Some((object, mut hit)) = first_hit(scene, ray) {
        if let MaterialType::Fixed = object.material.material_type {
            return object.material.albedo;
        }

        // figure out the next medium
        let into = hit.normal.dot(*ray.direction) < 0.0;
        let next_medium = if into {
            debug_assert_eq!(medium, object.material.outside);
            object.material.inside
        } else {
            hit.normal = -hit.normal;
            debug_assert_eq!(medium, object.material.inside);
            object.material.outside
        };

        // sample the next ray
        let refract_ratio = medium.index_of_refraction / next_medium.index_of_refraction;
        let sample = sample_direction(&ray, &hit, object.material.material_type, refract_ratio, rng);

        let mut result = Color::new(0.0, 0.0, 0.0);

        // add the light contributions
        match strategy {
            Strategy::Simple => {
                result += object.material.emission;
            }
            Strategy::SampleLights => {
                if specular {
                    result += object.material.emission;
                }

                if sample.diffuse_fraction != 0.0 {
                    let light_start = hit.point + (*hit.normal * SHADOW_BIAS);
                    let light_contribution = sample_lights(scene, light_start, medium, rng, &hit);
                    result += object.material.albedo * light_contribution * sample.diffuse_fraction;
                }
            }
        }

        // add the contribution of the next ray
        let next_ray = Ray {
            start: hit.point + (*sample.direction * SHADOW_BIAS),
            direction: sample.direction,
        };
        let next_medium = if sample.crosses_surface { next_medium } else { medium };
        let next_contribution = trace_ray(scene, strategy, &next_ray, rng, bounces_left - 1, sample.specular, next_medium);

        result += object.material.albedo * next_contribution * sample.weight;

        (hit.t, result)
    } else {
        (f32::INFINITY, scene.sky_emission)
    };

    color_exp(medium.volumetric_color, t) * result
}

struct SampleInfo {
    /// the direction of the next ray
    direction: Unit<Vec3>,
    /// the weight associated with the direction sampling, needs to be divided out of the contribution of the next ray
    weight: f32,

    /// whether this sample crosses the surface, used to determine the next medium
    crosses_surface: bool,
    /// whether this sample was the result of a specular event, used for light sampling
    specular: bool,

    /// the fraction of this surface that behaves diffuse, used for light sampling
    diffuse_fraction: f32,
}

fn sample_direction<R: Rng>(ray: &Ray, hit: &Hit, material_type: MaterialType, refract_ratio: f32, rng: &mut R) -> SampleInfo {
    match material_type {
        MaterialType::Fixed => panic!("Can't sample direction for MaterialType::Fixed"),
        MaterialType::Diffuse => {
            let disk = Vec2::from_slice(&UnitDisc.sample(rng));
            let direction = disk_to_hemisphere(disk, hit.normal);
            SampleInfo { weight: 0.5, diffuse_fraction: 1.0, specular: false, crosses_surface: false, direction }
        }
        MaterialType::Mirror => {
            let direction = reflect_direction(ray.direction, hit.normal);
            SampleInfo { weight: 1.0, diffuse_fraction: 0.0, specular: true, crosses_surface: false, direction }
        }
        MaterialType::Transparent => {
            let (crosses_surface, direction) = snells_law(ray.direction, hit.normal, refract_ratio);
            SampleInfo { weight: 1.0, diffuse_fraction: 0.0, specular: true, crosses_surface, direction }
        }
        MaterialType::DiffuseMirror(f) => {
            let mut sample = if rng.gen::<f32>() < f {
                sample_direction(ray, hit, MaterialType::Diffuse, refract_ratio, rng)
            } else {
                sample_direction(ray, hit, MaterialType::Mirror, refract_ratio, rng)
            };

            sample.diffuse_fraction = f;
            sample
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

/// Compute the outgoing direction according to
/// [Snell's law](https://en.wikipedia.org/wiki/Snell%27s_law#Vector_form),
/// including total internal reflection.
/// `vec` and `normal` should point in opposite directions.
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

    // this initialization is okay because we return None at the end if we didn't hit anything
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
    color == Color::new(0.0, 0.0, 0.0)
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
