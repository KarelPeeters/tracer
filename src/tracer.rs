use nalgebra::{Rotation3, Unit, Vector3};
use rand::distributions::Distribution;
use rand::Rng;
use rand_distr::UnitSphere;

use crate::geometry::{Hit, Ray, Shape, reflect};
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
pub struct Camera {
    pub position: Point3,
    pub direction: Unit<Vec3>,
    pub fov_vertical: f32,
    pub fov_horizontal: f32,
}

impl Camera {
    //TODO fix distortion on the top and bottom of the image and
    //     this also doesn't work for near-vertical camera directions yet
    pub fn ray(&self, width: f32, height: f32, xi: f32, yi: f32) -> Ray {
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

    pub fn trace<R: Rng>(&self, ray: &Ray, rand: &mut R, depth_left: usize) -> Option<Color> {
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
            total += self.trace(&ray, rand, depth_left - 1).unwrap_or(Color::new(0.0, 0.0, 0.0));

            total *= object.material.reflect_color;
            total += object.material.emission;

            Some(total)
        } else {
            Some(self.sky)
        }
    }
}

