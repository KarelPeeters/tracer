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
    fn first_hit(&self, objects: &[Object], ray: &Ray) -> Option<ObjectHit>;
}

#[derive(Debug)]
pub struct NoAccel;

impl Accel for NoAccel {
    fn first_hit(&self, objects: &[Object], ray: &Ray) -> Option<ObjectHit> {
        first_hit(objects, ray).map(|(index, hit)| ObjectHit { id: ObjectId::new(index), hit })
    }
}

/// We don't return [ObjectHit] since the indices may not be correct.
pub fn first_hit<'a>(objects: impl IntoIterator<Item=&'a Object>, ray: &Ray) -> Option<(usize, Hit)> {
    objects.into_iter().enumerate()
        .filter_map(|(index, object)| {
            object.intersect(ray).map(|hit| (index, hit))
        })
        .min_by_key(|(_, hit)| N32::from_inner(hit.t))
}

