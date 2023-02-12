use std::fmt::{Debug, Formatter};
use std::num::NonZeroU32;

use decorum::Total;
use itertools::{Itertools, partition};

use crate::common::aabb::AxisBox;
use crate::common::math::{Axis3, Axis3Owner, Point3};
use crate::common::scene::Object;
use crate::cpu::accel::{Accel, first_hit, ObjectId};
use crate::cpu::geometry::{ObjectHit, Ray};

/// Implementation following on https://jacco.ompf2.com/2022/04/13/how-to-build-a-bvh-part-1-basics/.
pub struct BVH {
    // we don't store ObjectId here to minimize size
    ids: Vec<u32>,
    nodes: Vec<Node>,
}

#[derive(Debug, Copy, Clone)]
struct Node {
    bound: AxisBox,
    kind: NodeKind,
}

#[derive(Debug, Copy, Clone)]
enum NodeKind {
    Leaf {
        start: u32,
        len: NonZeroU32,
    },
    Branch {
        left_index: u32,
    },
}

impl BVH {
    pub fn new(objects: &[Object]) -> Self {
        assert!(objects.len() < u32::MAX as usize);

        let len = match NonZeroU32::new(objects.len() as u32) {
            None => return BVH { ids: vec![], nodes: vec![] },
            Some(len) => len,
        };

        let mut builder = Builder {
            objects,
            ids: (0..len.get()).collect_vec(),
            nodes: vec![],
        };

        let root = builder.build_leaf(0, len);
        builder.nodes.push(root);
        builder.split(0);

        BVH {
            ids: builder.ids,
            nodes: builder.nodes,
        }
    }

    fn first_hit_impl(&self, objects: &[Object], ray: &Ray, node: u32, mut t_max: f32) -> Option<ObjectHit> {
        let node = self.nodes[node as usize];

        if !node.bound.intersects(ray) {
            return None;
        }

        match node.kind {
            NodeKind::Leaf { start, len } => {
                let objects = &objects[start as usize..][..len.get() as usize];
                first_hit(objects, ray).map(|(index, hit)| {
                    ObjectHit {
                        id: ObjectId::new(start as usize + index),
                        hit,
                    }
                })
            }
            NodeKind::Branch { left_index } => {
                let left = self.first_hit_impl(objects, ray, left_index, t_max);
                if let Some(left) = left.as_ref() {
                    t_max = t_max.min(left.hit.t);
                }
                let right = self.first_hit_impl(objects, ray, left_index + 1, t_max);
                ObjectHit::closest_option(left, right)
            }
        }
    }
}

impl Accel for BVH {
    fn first_hit(&self, objects: &[Object], ray: &Ray) -> Option<ObjectHit> {
        if self.nodes.is_empty() {
            return None;
        }

        // TODO consider making t_max part of Ray everywhere
        self.first_hit_impl(objects, ray, 0, f32::INFINITY)
    }
}

impl AxisBox {
    pub fn intersects(self, ray: &Ray) -> bool {
        let mut t_min = f32::NEG_INFINITY;
        let mut t_max = f32::INFINITY;

        for axis in Axis3::ALL {
            let t1 = (self.low.get(axis) - ray.start.get(axis)) / ray.direction.get(axis);
            let t2 = (self.high.get(axis) - ray.start.get(axis)) / ray.direction.get(axis);
            t_min = t_min.max(t1.min(t2));
            t_max = t_max.min(t1.max(t2));
        }

        t_max >= t_min && t_max > 0.0
    }
}

struct Builder<'a> {
    objects: &'a [Object],
    ids: Vec<u32>,
    nodes: Vec<Node>,
}

impl Builder<'_> {
    fn compute_bound(&self, start: u32, len: NonZeroU32) -> AxisBox {
        (start..start + len.get())
            .map(|id| AxisBox::for_object(&self.objects[id as usize]))
            .reduce(AxisBox::combine)
            .unwrap()
    }

    fn build_leaf(&self, start: u32, len: NonZeroU32) -> Node {
        let bound = self.compute_bound(start, len);
        Node { bound, kind: NodeKind::Leaf { start, len } }
    }

    fn split(&mut self, node_index: u32) {
        let node = &self.nodes[node_index as usize];
        let bound = node.bound;
        let (start, len) = match node.kind {
            NodeKind::Leaf { start, len } => (start, len),
            NodeKind::Branch { .. } => panic!("can only split leaf nodes"),
        };

        // find the split axis and point
        let extend = bound.high - bound.low;
        let split_axis = Axis3::ALL.into_iter()
            .max_by_key(|&a| Total::from_inner(extend.get(a)))
            .unwrap();
        let split_value = (bound.low.get(split_axis) + bound.high.get(split_axis)) / 2.0;

        // rearrange the objects
        let split_index = partition(
            &mut self.ids[start as usize..][..len.get() as usize],
            |&id| object_centroid(&self.objects[id as usize]).get(split_axis) < split_value,
        ) as u32;

        // push child nodes
        let left_len = NonZeroU32::new(split_index);
        let right_len = NonZeroU32::new(len.get() - split_index);
        let (left_len, right_len) = match (left_len, right_len) {
            // both children will actually contain stuff
            (Some(left_len), Some(right_len)) => (left_len, right_len),
            // we failed to actually split anything
            (None, _) | (_, None) => return,
        };

        let left = self.build_leaf(start, left_len);
        let right = self.build_leaf(start + left_len.get(), right_len);
        let left_index = self.nodes.len() as u32;
        self.nodes.push(left);
        self.nodes.push(right);

        // fix current node
        self.nodes[node_index as usize].kind = NodeKind::Branch { left_index };

        // continue recursing
        self.split(left_index);
        self.split(left_index + 1);
    }
}

// TODO figure out what centroid to use, does it need to be correct or is best-effort fine?
//   can we just use the object BB centroid?
fn object_centroid(object: &Object) -> Point3 {
    let b = AxisBox::for_object(object);
    b.low.middle(b.high)
}

impl Debug for BVH {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BVH(ids={}, nodes={})", self.ids.len(), self.nodes.len())
    }
}

#[cfg(test)]
mod test {
    use crate::common::aabb::AxisBox;
    use crate::common::math::{Point3, Vec3};
    use crate::cpu::geometry::Ray;

    #[test]
    fn aabb_intersect() {
        let aabb = AxisBox::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0));
        let ray = Ray {
            start: Point3::new(0.0, 0.0, -4.0),
            direction: Vec3::z_axis(),
        };
        assert!(aabb.intersects(&ray));
    }
}
