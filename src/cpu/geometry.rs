#![allow(dead_code)]

use std::fmt::{Debug, Formatter};
use std::ops::Mul;

use crate::common::scene::{Object, Point3, Shape, Transform, Vec3};
use nalgebra::Unit;

pub struct PrettyVec { x: f32, y: f32, z: f32 }

impl PrettyVec {
    fn from_vec(v: &Vec3) -> PrettyVec { PrettyVec { x: v.x, y: v.y, z: v.z } }
    fn from_point(v: &Point3) -> PrettyVec { PrettyVec { x: v.x, y: v.y, z: v.z } }
    fn from_unit(v: &Unit<Vec3>) -> PrettyVec { PrettyVec { x: v.x, y: v.y, z: v.z } }
}

impl Debug for PrettyVec {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

#[derive(Clone)]
pub struct Ray {
    pub start: Point3,
    pub direction: Unit<Vec3>,
}

impl Debug for Ray {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ray")
            .field("start", &PrettyVec::from_point(&self.start))
            .field("direction", &PrettyVec::from_unit(&self.direction))
            .finish()
    }
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
            start: self.transform_point(&rhs.start),
            direction: Unit::new_normalize(self.transform_vector(rhs.direction.as_ref())),
        }
    }
}

pub struct Hit {
    pub t: f32,
    pub point: Point3,
    pub normal: Unit<Vec3>,
}

impl Debug for Hit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hit")
            .field("t", &self.t)
            .field("point", &PrettyVec::from_point(&self.point))
            .field("normal", &PrettyVec::from_unit(&self.normal))
            .finish()
    }
}

impl Hit {
    fn transform(&self, transform: &Transform, direction: Unit<Vec3>) -> Hit {
        let inv_transpose = Transform::from_matrix_unchecked(transform.inverse().into_inner().transpose());

        Hit {
            t: self.t / (transform * direction.as_ref()).norm(),
            point: transform.transform_point(&self.point),
            normal: Unit::new_normalize(inv_transpose.transform_vector(&self.normal)),
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
        point: point.clone(),
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

        hit.map(|hit| hit.transform(&self.transform, ray.direction.clone()))
    }
}
