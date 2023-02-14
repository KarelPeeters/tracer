use std::fmt::Debug;

use decorum::N32;
use derive_more::Constructor;

use crate::common::scene::Object;
use crate::cpu::geometry::{Hit, ObjectHit, Ray};
use crate::cpu::geometry::Intersect;

pub mod octree;
pub mod bvh;

/// A stable index into `sccene.objects`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Constructor)]
pub struct ObjectId {
    pub index: usize,
}

pub trait Accel: Debug {
    fn first_hit(&self, objects: &[Object], ray: &Ray, filter: impl Fn(&Object) -> bool) -> Option<ObjectHit>;
}

#[derive(Debug)]
pub struct NoAccel;

impl Accel for NoAccel {
    fn first_hit(&self, objects: &[Object], ray: &Ray, filter: impl Fn(&Object) -> bool) -> Option<ObjectHit> {
        first_hit(objects, ray, filter).map(|(index, hit)| ObjectHit { id: ObjectId::new(index), hit })
    }
}

/// We don't return [ObjectHit] since the indices may not be correct.
pub fn first_hit<'a>(objects: impl IntoIterator<Item=&'a Object>, ray: &Ray, filter: impl Fn(&Object) -> bool) -> Option<(usize, Hit)> {
    objects.into_iter().enumerate()
        .filter_map(|(index, object)| {
            if filter(object) {
                object.intersect(ray).map(|hit| (index, hit))
            } else {
                None
            }
        })
        .min_by_key(|(_, hit)| N32::from_inner(hit.t))
}

