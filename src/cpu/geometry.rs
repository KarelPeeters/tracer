#![allow(dead_code)]

use std::fmt::{Debug, Formatter};
use std::ops::Mul;

use more_asserts::{assert_lt, debug_assert_lt};
use nalgebra::Unit;
use rand::distributions::Distribution;
use rand::Rng;
use rand_distr::UnitSphere;

use crate::common::scene::{Object, Point3, Shape, Transform, Vec3};

pub struct PrettyVec { x: f32, y: f32, z: f32 }

impl PrettyVec {
    pub fn from_vec(v: &Vec3) -> PrettyVec { PrettyVec { x: v.x, y: v.y, z: v.z } }
    pub fn from_point(v: &Point3) -> PrettyVec { PrettyVec { x: v.x, y: v.y, z: v.z } }
    pub fn from_unit(v: &Unit<Vec3>) -> PrettyVec { PrettyVec { x: v.x, y: v.y, z: v.z } }
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

fn sphere_intersect(ray: &Ray) -> Option<Hit> {
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

    let mut point = ray.at(t);
    point.coords.normalize_mut(); //renormalize for better accuracy

    if point != point {
        return None;
    }

    Some(Hit {
        t,
        point: point.clone(),
        normal: Unit::new_unchecked(point.coords),
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
        (0.0 <= x && x < 1.0) && (0.0 <= y && y < 1.0) && (x + y < 1.0)
    })
}

fn cylinder_intersect(ray: &Ray) -> Option<Hit> {
    //work in xz plane
    let start = ray.start.xz();
    let (direction, dir_2d_norm) = Unit::new_and_get(ray.direction.xz());

    let b: f32 = start.coords.dot(&direction);
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
    let normal = Unit::new_normalize(Vec3::new(point.x, 0.0, point.z));
    point.x = normal.x; //renormalize point for better accuracy
    point.z = normal.z;

    if point != point {
        return None;
    };

    Some(Hit { t, point, normal })
}

pub trait Intersect {
    fn intersect(&self, ray: &Ray) -> Option<Hit>;

    fn area_seen_from(&self, from: &Point3) -> f32;

    fn area(&self) -> f32;

    fn sample<R: Rng>(&self, rng: &mut R) -> (f32, Point3);
}

impl Intersect for Object {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let obj_ray = &self.transform.inverse() * ray;

        let obj_hit = match self.shape {
            Shape::Sphere => sphere_intersect(&obj_ray),
            Shape::Plane => plane_intersect(&obj_ray),
            Shape::Triangle => triangle_intersect(&obj_ray),
            Shape::Cylinder => cylinder_intersect(&obj_ray),
        };

        check_hit(&obj_hit);
        let world_hit = obj_hit.map(|hit| hit.transform(&self.transform, ray.direction.clone()));
        check_hit(&world_hit);

        world_hit
    }

    fn area_seen_from(&self, from: &Point3) -> f32 {
        assert_eq!(self.shape, Shape::Sphere);

        let dist = (self.transform.inverse() * from).coords.norm();
        let delta = 2.0 * (1f32 / dist).asin();
        return delta * delta / 4.0 / std::f32::consts::PI;
    }

    fn area(&self) -> f32 {
        assert_eq!(self.shape, Shape::Sphere);

        return 4.0 * std::f32::consts::PI;
    }

    fn sample<R: Rng>(&self, rng: &mut R) -> (f32, Point3) {
        assert_eq!(self.shape, Shape::Sphere);

        let vec = Vec3::from_column_slice(&UnitSphere.sample(rng));
        //TODO 2.0 is not exactly the correct weight because not exactly half of the sphere is visible
        (2.0, self.transform * Point3::from(vec))
    }
}

fn check_hit(hit: &Option<Hit>) {
    if let Some(hit) = hit {
        debug_assert!(hit.t >= 0.0);
        debug_assert!(hit.normal == hit.normal);
        debug_assert!(hit.point == hit.point);
    }
}