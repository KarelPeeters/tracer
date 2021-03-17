#![allow(dead_code)]

use std::fmt::Debug;
use std::ops::Mul;

use rand::distributions::Distribution;
use rand::Rng;
use rand_distr::UnitSphere;

use crate::common::math::{Norm, Point2, Point3, Transform, Unit, Vec2, Vec3};
use crate::common::scene::{Object, Shape};

#[derive(Copy, Clone, Debug)]
pub struct Ray {
    pub start: Point3,
    pub direction: Unit<Vec3>,
}

impl Ray {
    pub fn new(start: Point3, direction: Unit<Vec3>) -> Ray {
        Ray { start, direction }
    }

    pub fn at(&self, t: f32) -> Point3 {
        self.start + *self.direction * t
    }
}

impl Mul<&Ray> for Transform {
    type Output = Ray;

    fn mul(self, rhs: &Ray) -> Self::Output {
        Ray {
            start: self * rhs.start,
            direction: (self * *rhs.direction).normalized(),
        }
    }
}

#[derive(Debug)]
pub struct Hit {
    pub t: f32,
    pub point: Point3,
    pub normal: Unit<Vec3>,
}

impl Hit {
    fn transform(&self, transform: Transform, direction: Unit<Vec3>) -> Hit {
        Hit {
            t: self.t * (transform * (*direction)).norm(),
            point: transform * self.point,
            normal: transform.inv_transpose_mul(*self.normal).normalized(),
        }
    }
}

fn sphere_intersect(ray: &Ray) -> Option<Hit> {
    let b: f32 = ray.start.coords().dot(*ray.direction);
    let c: f32 = ray.start.coords().norm_squared() - 1.0;

    let d = b * b - c;
    if d < 0.0 || (c > 0.0 && b > 0.0) {
        return None;
    }

    let t_near = -b - d.sqrt();
    let t_far = -b + d.sqrt();

    let t = if t_near >= 0.0 {
        t_near
    } else {
        t_far
    };

    //renormalize for better accuracy and bail if zero
    let result = ray.at(t).coords().try_normalized()?;

    Some(Hit {
        t,
        point: Point3::from_coords(*result),
        normal: result,
    })
}

fn plane_intersect(ray: &Ray) -> Option<Hit> {
    let t = -ray.start.z / ray.direction.z;

    if !t.is_finite() || t < 0.0 {
        None
    } else {
        Some(Hit {
            t,
            point: ray.at(t),
            normal: Vec3::z_axis(),
        })
    }
}

fn triangle_intersect(ray: &Ray) -> Option<Hit> {
    plane_intersect(ray).filter(|hit| {
        let x = hit.point.x;
        let y = hit.point.y;
        (0.0..1.0).contains(&x) && (0.0..1.0).contains(&y) && (x + y < 1.0)
    })
}

fn cylinder_intersect(ray: &Ray) -> Option<Hit> {
    //work in xz plane
    let start = Point2::new(ray.start.x, ray.start.z);
    let (direction, dir_2d_norm) = Vec2::new(ray.direction.x, ray.direction.z).normalized_and_get();

    let b: f32 = start.coords.dot(*direction);
    let c: f32 = start.coords.norm_squared() - 1.0;

    let d = b * b - c;
    if d < 0.0 || (c > 0.0 && b > 0.0) {
        return None;
    }

    let t_near = -b - d.sqrt();
    let t_far = -b + d.sqrt();

    let t = if t_near >= 0.0 {
        t_near
    } else {
        t_far
    };

    //scale back to 3D
    let t = t / dir_2d_norm;

    let mut point = ray.at(t);
    let normal = Vec3::new(point.x, 0.0, point.z).normalized();
    point.x = normal.x; //renormalize point for better accuracy
    point.z = normal.z;

    if point != point {
        return None;
    };

    Some(Hit { t, point, normal })
}

pub trait Intersect {
    fn intersect(&self, ray: &Ray) -> Option<Hit>;

    fn area_seen_from(&self, from: Point3) -> f32;

    fn area(&self) -> f32;

    fn sample<R: Rng>(&self, rng: &mut R) -> (f32, Point3);
}

impl Intersect for Object {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let obj_ray = self.transform.inv() * ray;

        let obj_hit = match self.shape {
            Shape::Sphere => sphere_intersect(&obj_ray),
            Shape::Plane => plane_intersect(&obj_ray),
            Shape::Triangle => triangle_intersect(&obj_ray),
            Shape::Cylinder => cylinder_intersect(&obj_ray),
        };

        check_hit(&obj_hit);
        let world_hit = obj_hit.map(|hit| hit.transform(self.transform, ray.direction));
        check_hit(&world_hit);

        world_hit
    }

    fn area_seen_from(&self, from: Point3) -> f32 {
        assert_eq!(self.shape, Shape::Sphere);

        let dist = (self.transform.inv() * from).distance_to(Point3::origin());
        let delta = 2.0 * clamp(1.0 / dist, -1.0, 1.0).asin();

        delta * delta / 4.0 / std::f32::consts::PI
    }

    fn area(&self) -> f32 {
        assert_eq!(self.shape, Shape::Sphere);

        4.0 * std::f32::consts::PI
    }

    fn sample<R: Rng>(&self, rng: &mut R) -> (f32, Point3) {
        assert_eq!(self.shape, Shape::Sphere);

        let vec = Vec3::from_slice(&UnitSphere.sample(rng));
        //TODO 2.0 is not exactly the correct weight because not exactly half of the sphere is visible
        (2.0, self.transform * (Point3::origin() + vec))
    }
}

fn check_hit(hit: &Option<Hit>) {
    if let Some(hit) = hit {
        debug_assert!(hit.t >= 0.0);
        debug_assert!(hit.normal == hit.normal);
        debug_assert!(hit.point == hit.point);
    }
}

fn clamp(x: f32, min: f32, max: f32) -> f32 {
    debug_assert!(min <= max);
    x.min(max).max(min)
}