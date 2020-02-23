use nalgebra::Unit;

use crate::{Point3, Vec3};

#[derive(Debug)]
pub struct Ray {
    pub start: Point3,
    pub direction: Unit<Vec3>,
}

impl Ray {
    pub fn new(start: &Point3, direction: &Unit<Vec3>) -> Ray {
        Ray {
            start: start.clone(),
            direction: direction.clone(),
        }
    }

    pub fn at(&self, t: f32) -> Point3 {
        &self.start + self.direction.scale(t)
    }
}

pub struct Hit {
    pub t: f32,
    pub point: Point3,
    pub normal: Unit<Vec3>,
}

trait Intersect {
    fn intersect(&self, ray: &Ray) -> Option<Hit>;
}

#[derive(Debug)]
pub struct Sphere {
    pub center: Point3,
    pub radius: f32,
}

impl Intersect for Sphere {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let rel = &ray.start - &self.center;

        //solve quadratic equation
        let b: f32 = 2.0 * rel.dot(&ray.direction);
        let c = rel.norm_squared() - self.radius * self.radius;

        let d = b * b - 4.0 * c;
        if d < 0.0 {
            return None;
        }

        let t1 = (-b + d.sqrt()) / 2.0;
        let t2 = (-b - d.sqrt()) / 2.0;

        //find closest solution in front of the ray
        if t1 < 0.0 && t2 < 0.0 {
            return None;
        }
        let t = t1.min(t2);

        //construct intersection
        let point = ray.at(t);

        Some(Hit {
            t,
            point,
            normal: Unit::new_normalize(&point - &self.center),
        })
    }
}

#[derive(Debug)]
pub struct Plane {
    pub point: Point3,
    pub normal: Unit<Vec3>,
}

impl Intersect for Plane {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let rel = &ray.start - &self.point;

        let num = self.normal.dot(&rel);
        let den = self.normal.dot(&ray.direction);
        let t = -num / den;

        if !t.is_finite() || t < 0.0 {
            None
        } else {
            Some(Hit {
                t,
                point: ray.at(t),
                normal: self.normal.clone(),
            })
        }
    }
}

#[derive(Debug)]
pub struct Triangle {
    pub base: Point3,
    pub da: Vec3,
    pub db: Vec3,

    pub normal: Unit<Vec3>,
    // pub base_normal: Vec3,
    // pub a_normal: Vec3,
    // pub b_normal: Vec3,
}

impl Intersect for Triangle {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let plane = Plane {
            point: self.base.clone(),
            normal: self.normal.clone(),
        };

        plane.intersect(ray).and_then(|mut hit| {
            let p = &hit.point - &self.base;
            let ua = self.da.dot(&p) / self.da.norm_squared();
            let ub = self.db.dot(&p) / self.db.norm_squared();

            if (0.0 <= ua && ua <= 1.0) && (0.0 <= ub && ub <= 1.0) && (ua + ub <= 1.0) {
                hit.normal = self.normal.clone();
                Some(hit)
            } else {
                None
            }
        })
    }
}

#[derive(Debug)]
pub enum Shape {
    Sphere(Sphere),
    Plane(Plane),
    Triangle(Triangle),
}

impl Shape {
    pub fn intersect(&self, ray: &Ray) -> Option<Hit> {
        match self {
            Shape::Sphere(s) => s.intersect(ray),
            Shape::Plane(s) => s.intersect(ray),
            Shape::Triangle(s) => s.intersect(ray),
        }
    }
}

pub fn reflect(vec: &Unit<Vec3>, normal: &Unit<Vec3>) -> Unit<Vec3> {
    Unit::new_unchecked(vec.as_ref() - &normal.scale(2.0 * vec.dot(normal)))
}
