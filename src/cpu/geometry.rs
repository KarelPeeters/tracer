#![allow(dead_code)]

use std::ops::Mul;

use nalgebra::Unit;

use crate::common::scene::{Object, Point3, Shape, Transform, Vec3};

#[derive(Debug, Clone)]
pub struct Ray {
    pub start: Point3,
    pub direction: Unit<Vec3>,
}

impl Ray {
    pub fn new(start: &Point3, direction: &Unit<Vec3>) -> Ray {
        Ray {
            start: start.clone(),
            direction: (*direction).clone(),
        }
    }

    pub fn at(&self, t: f32) -> Point3 {
        &self.start + self.direction.scale(t)
    }
}

impl Mul<&Ray> for &Transform {
    type Output = Ray;

    fn mul(self, rhs: &Ray) -> Self::Output {
        Ray {
            start: self * rhs.start,
            direction: Unit::new_normalize(self * rhs.direction.as_ref()),
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
    fn transform(&self, transform: &Transform, direction: Unit<Vec3>) -> Hit {
        Hit {
            t: self.t / (transform * direction.as_ref()).norm(),
            point: transform * &self.point,
            normal: Unit::new_normalize(transform.inverse_transform_vector(&self.normal)),
        }
    }
}

fn sphere_intersect(ray: Ray) -> Option<Hit> {
    let b: f32 = ray.start.coords.dot(&ray.direction);
    let c: f32 = ray.start.coords.norm_squared() - 1.0;

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

    let point = ray.at(t);
    Some(Hit {
        t,
        point,
        normal: Unit::new_unchecked(point.coords),
    })
}

fn plane_intersect(ray: Ray) -> Option<Hit> {
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

fn triangle_intersect(ray: Ray) -> Option<Hit> {
    plane_intersect(ray).filter(|hit| {
        let x = hit.point.x;
        let y = hit.point.y;
        (0.0 <= x && x < 1.0) && (0.0 <= y && y < 1.0) && (x + y < 1.0)
    })
}

pub trait Intersect {
    fn intersect(&self, ray: &Ray) -> Option<Hit>;
}

impl Intersect for Object {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let obj_ray = &self.transform.inverse() * ray;

        let hit = match self.shape {
            Shape::Sphere => sphere_intersect(obj_ray),
            Shape::Plane => plane_intersect(obj_ray),
            Shape::Triangle => triangle_intersect(obj_ray),
        };

        if let Some(hit) = &hit {
            debug_assert!(hit.t >= 0.0);
        }

        hit.map(|hit| hit.transform(&self.transform, ray.direction))
    }
}
