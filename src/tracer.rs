use image::{ImageBuffer, Rgba};
use nalgebra::{Unit, Vector3};
use palette::{Alpha, LinSrgba, Srgba};
use rand::{Rng, thread_rng};
use rand::distributions::{Distribution, Uniform};
use rand_distr::UnitSphere;
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::camera::Camera;
use crate::geometry::{Hit, Ray, reflect, Shape};
use crate::material::Material;

pub type Vec3 = Vector3<f32>;
pub type Point3 = nalgebra::Point3<f32>;
pub type Color = palette::LinSrgb;


#[derive(Debug)]
pub struct Object {
    pub shape: Shape,
    pub material: Material,
}

#[derive(Debug)]
pub struct Light {
    pub position: Point3,
    pub color: Color,
}

#[derive(Debug)]
pub struct Scene {
    pub objects: Vec<Object>,
    pub lights: Vec<Light>,
    pub sky: Color,
}

const SHADOW_BIAS: f32 = 0.0001;

impl Scene {
    fn first_hit_ray(&self, ray: &Ray) -> Option<(&Object, Hit)> {
        self.objects.iter()
            .flat_map(|s| s.shape.intersect(&ray).map(|h| (s, h)))
            .min_by(|(_, ah), (_, bh)| ah.t.partial_cmp(&bh.t).expect("t == NaN"))
    }

    fn first_hit(&self, start: &Point3, direction: Unit<Vec3>) -> Option<(&Object, Hit)> {
        let mut ray = Ray { start: start.clone(), direction };
        ray.start = ray.at(SHADOW_BIAS);
        self.first_hit_ray(&ray)
    }

    pub fn trace_ray<R: Rng>(&self, ray: &Ray, rand: &mut R, depth_left: usize) -> Option<Color> {
        if depth_left == 0 { return None; }

        if let Some((object, hit)) = self.first_hit_ray(ray) {
            let mut total = Color::new(0.0, 0.0, 0.0);

            //to lights
            for light in &self.lights {
                let (direction, light_t) = Unit::new_and_get(&light.position - &hit.point);

                //if there's something blocking the light, continue
                if let Some((_, other_hit)) = self.first_hit(&hit.point, direction) {
                    if other_hit.t < light_t { continue; }
                }

                //add light, after inverse-square law
                total += light.color / (light_t * light_t)
            }

            //to other objects
            let reflect_direction = if object.material.diff_prob.sample(rand) {
                //diffuse
                let data: [f32; 3] = UnitSphere.sample(rand);
                Unit::new_unchecked(Vector3::new(data[0], data[1], data[2]))
            } else {
                //mirror
                reflect(&ray.direction, &hit.normal)
            };

            let mut ray = Ray::new(&hit.point, &reflect_direction);
            ray.start = ray.at(SHADOW_BIAS);

            //TODO think about the bailout condition
            total += self.trace_ray(&ray, rand, depth_left - 1)
                .unwrap_or(Color::new(0.0, 0.0, 0.0));

            total *= object.material.reflect_color;
            total += object.material.emission;

            Some(total)
        } else {
            Some(self.sky)
        }
    }
}

pub fn trace_image<C: Camera + Sync>(
    scene: &Scene,
    camera: &C,
    width: u32,
    height: u32,
    max_depth: usize,
    sample_count: usize,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
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

                let color: LinSrgba = scene.trace_ray(&ray, &mut rand, max_depth)
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

    image
}


