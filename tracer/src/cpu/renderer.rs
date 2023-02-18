use std::cmp::max;

use rand::distributions::Distribution;
use rand::Rng;
use rand_distr::UnitDisc;

use crate::common::math::{Norm, Point3, Transform, Unit, Vec2, Vec3};
use crate::common::progress::PixelResult;
use crate::common::scene::{Camera, Color, MaterialType, Medium, Object, Scene};
use crate::cpu::accel::{Accel, ObjectId};
use crate::cpu::geometry::{Hit, Intersect, ObjectHit, Ray};
use crate::cpu::stats::ColorVarianceEstimator;

#[derive(Debug, Copy, Clone)]
pub struct CpuRenderSettings {
    pub stop_condition: StopCondition,
    pub max_bounces: u32,
    pub anti_alias: bool,
    pub strategy: Strategy,
    pub octree_max_flat_size: usize,
}

#[derive(Debug, Copy, Clone)]
pub enum StopCondition {
    SampleCount(u32),
    // TODO consider variance in neighborhood instead of only single pixel
    // TODO rel_var should really be `var / mag / "difference in color between neighboring pixels"`,
    //   currently we just end up focusing on edges instead of surfaces
    Variance { min_samples: u32, max_relative_variance: f32 },
}

#[derive(Debug, Copy, Clone)]
pub enum Strategy {
    Simple,
    SampleLights,
}

pub struct RenderStructure<'a, A> {
    pub scene: &'a Scene,
    pub camera: RayCamera,
    pub accel: A,
    pub lights: Vec<ObjectId>,
    pub settings: CpuRenderSettings,
}

impl<A: Accel> RenderStructure<'_, A> {
    pub fn calculate_pixel(&self, rng: &mut impl Rng, x: u32, y: u32) -> PixelResult {
        let settings = &self.settings;
        let scene = self.scene;

        let mut estimator = ColorVarianceEstimator::default();

        while !settings.stop_condition.is_done(&estimator) {
            let ray = self.camera.ray(rng, x, y);
            let color = trace_ray(
                scene,
                &self.accel,
                &self.lights,
                settings.strategy,
                &ray,
                true,
                rng,
                settings.max_bounces,
                true,
                scene.camera.medium,
            );
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

pub fn is_light(object: &Object) -> bool {
    !is_black(object.material.emission)
}

fn sample_lights<R: Rng>(scene: &Scene, accel: &impl Accel, lights: &[ObjectId], next_start: Point3, medium: Medium, rng: &mut R, hit: &Hit) -> Color {
    let mut result = Color::new(0.0, 0.0, 0.0);

    for &light_id in lights {
        let light = &scene.objects[light_id.index];
        assert!(is_light(light));

        let (weight, target) = light.sample(rng);
        let light_ray = Ray { start: next_start, direction: (target - next_start).normalized() };

        // TODO is this actually correct for transparent objects ?
        match accel.first_hit(&scene.objects, &light_ray, filter_fixed_camera_only(false)) {
            // the light is unobstructed, it's the first thing we hit again
            Some(ObjectHit { id: object, hit: light_hit }) if object == light_id => {
                let abs_cos = light_ray.direction.dot(*hit.normal).abs();
                let volumetric_mask = color_exp(medium.volumetric_color, light_hit.t);

                result += light.material.emission * weight * abs_cos * volumetric_mask * light.area_seen_from(next_start);
            }
            // another object is blocking the light
            Some(_) => {}
            // hit nothing, should means we missed the edge of the light because of numerical issues
            None => {}
        }
    }

    result
}

fn filter_fixed_camera_only(is_camera_ray: bool) -> impl Fn(&Object) -> bool {
    move |o: &Object| {
        match o.material.material_type {
            MaterialType::Fixed {  camera_only } => is_camera_ray || !camera_only,
            _ => true,
        }
    }
}

fn trace_ray<'a, R: Rng>(
    scene: &Scene,
    accel: &'a impl Accel,
    lights: &[ObjectId],
    strategy: Strategy,
    ray: &Ray,
    is_camera_ray: bool,
    rng: &mut R,
    bounces_left: u32,
    specular: bool,
    medium: Medium,
) -> Color {
    if bounces_left == 0 {
        return Color::new(0.0, 0.0, 0.0);
    }

    let filter = filter_fixed_camera_only(is_camera_ray);
    let (t, result) = if let Some(object_hit) = accel.first_hit(&scene.objects, ray, filter) {
        let ObjectHit { id: object, mut hit } = object_hit;
        let object = &scene.objects[object.index];

        if let MaterialType::Fixed { camera_only } = object.material.material_type {
            debug_assert!(is_camera_ray || !camera_only);
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
                    let light_contribution = sample_lights(scene, accel, lights, light_start, medium, rng, &hit);
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
        let next_contribution = trace_ray(scene, accel, lights, strategy, &next_ray, false, rng, bounces_left - 1, sample.specular, next_medium);

        result += object.material.albedo * next_contribution * sample.weight;

        (hit.t, result)
    } else {
        (f32::INFINITY, scene.sky_emission)
    };

    color_exp(medium.volumetric_color, t) * result
}

#[derive(Debug)]
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
        MaterialType::Fixed { .. } => panic!("Can't sample direction for {material_type:?}"),
        MaterialType::Diffuse => {
            // cosine weighed sampling from the hemisphere pointing towards hit.normal
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
    let result = Unit::new_unchecked(Vec3::new(disk.x, disk.y, z));
    if result.dot(*normal) >= 0.0 {
        result
    } else {
        -result
    }
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

#[cfg(test)]
mod test {
    use crate::common::math::{Vec2, Vec3};
    use crate::cpu::renderer::disk_to_hemisphere;

    #[test]
    fn disk_to_hemisphere_z() {
        let disk = Vec2::new(0.1, 0.1);
        let normal = Vec3::z_axis();
        let result = disk_to_hemisphere(disk, normal);
        println!("{:?}", result);
    }
}