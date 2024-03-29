use std::fmt::{Debug, Formatter};
use std::num::NonZeroU32;

use decorum::Total;
use itertools::{Itertools, partition};

use crate::common::aabb::AxisBox;
use crate::common::math::{Axis3, Axis3Owner, lerp, Point3};
use crate::common::scene::Object;
use crate::cpu::accel::{Accel, first_hit, ObjectId};
use crate::cpu::geometry::{ObjectHit, Ray};

/// Implementation following
/// * https://jacco.ompf2.com/2022/04/13/how-to-build-a-bvh-part-1-basics/.
/// * https://jacco.ompf2.com/2022/04/18/how-to-build-a-bvh-part-2-faster-rays/
pub struct BVH {
    /// objects with infinite spans that don't fit in the tree structure
    global_ids: Vec<SmallId>,
    /// the tree objects
    ids: Vec<SmallId>,
    /// the tree nodes
    nodes: Vec<Node>,
}

pub enum BVHSplitStrategy {
    SplitLargestAxis,
    SurfaceAreaHeuristic { test_planes: Option<usize> },
}

impl Default for BVHSplitStrategy {
    fn default() -> Self {
        // TODO investigate why the other options are slower (at least on the "random tiles" scene)
        BVHSplitStrategy::SplitLargestAxis
    }
}

#[derive(Debug, Clone)]
struct Node {
    bound: AxisBox,
    kind: NodeKind,
}

/// Smaller version of ObjectId to fit more things into the cache.
#[derive(Debug, Copy, Clone)]
struct SmallId {
    index: u32,
}

#[derive(Debug, Clone)]
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
    pub fn new(objects: &[Object], strategy: BVHSplitStrategy) -> Self {
        assert!(objects.len() < u32::MAX as usize);
        let total_len = objects.len() as u32;

        let mut ids = (0..total_len).map(|id| SmallId { index: id }).collect_vec();
        // TODO also check for non-finite transforms?
        let global_start = partition(&mut ids, |&id| AxisBox::for_shape(objects[id.index as usize].shape).is_finite());
        let global_ids = ids.split_off(global_start);

        let len = match NonZeroU32::new(ids.len() as u32) {
            None => return BVH { global_ids, ids: vec![], nodes: vec![] },
            Some(len) => len,
        };

        let mut builder = Builder {
            strategy,
            objects,
            ids,
            nodes: vec![],
        };

        let root = builder.build_leaf(0, len);
        builder.nodes.push(root);
        builder.split(0);

        builder.check(&global_ids);

        BVH {
            global_ids,
            ids: builder.ids,
            nodes: builder.nodes,
        }
    }

    fn first_hit_impl(&self, objects: &[Object], ray: &Ray, filter: &impl Fn(&Object) -> bool, node: u32, mut t_max: f32) -> Option<ObjectHit> {
        let node = &self.nodes[node as usize];

        if node.bound.intersects(ray).is_none() {
            return None;
        }

        match node.kind {
            NodeKind::Leaf { start, len } => {
                let objects = (start..start + len.get()).map(|index| &objects[self.ids[index as usize].index as usize]);
                first_hit(objects, ray, filter).map(|(index, hit)| {
                    let id = self.ids[start as usize + index].to_large();
                    ObjectHit { id, hit }
                })
            }
            NodeKind::Branch { left_index } => {
                let mut first_index = left_index;
                let mut second_index = left_index + 1;
                let mut first_t = self.nodes[first_index as usize].bound.intersects(ray).unwrap_or(f32::INFINITY);
                let mut second_t = self.nodes[second_index as usize].bound.intersects(ray).unwrap_or(f32::INFINITY);

                // TODO why does simplifying this make everything 2x slower?
                if !(first_t < second_t) {
                    std::mem::swap(&mut first_index, &mut second_index);
                    std::mem::swap(&mut first_t, &mut second_t);
                }

                let mut best = None;

                if first_t < t_max {
                    let first = self.first_hit_impl(objects, ray, filter, first_index, t_max);
                    t_max = f32::min(t_max, first.as_ref().map_or(f32::INFINITY, |hit| hit.hit.t));
                    best = ObjectHit::closest_option(best, first);
                }
                if second_t < t_max {
                    let second = self.first_hit_impl(objects, ray, filter, second_index, t_max);
                    best = ObjectHit::closest_option(best, second);
                }

                best
            }
        }
    }
}

impl Accel for BVH {
    fn first_hit(&self, objects: &[Object], ray: &Ray, filter: impl Fn(&Object) -> bool) -> Option<ObjectHit> {
        let global_objects = self.global_ids.iter().map(|id| &objects[id.index as usize]);
        let global_hit = first_hit(global_objects, ray, &filter)
            .map(|(index, hit)| ObjectHit { id: self.global_ids[index].to_large(), hit });

        if self.nodes.is_empty() {
            return global_hit;
        }

        // TODO consider making t_max part of Ray everywhere
        let t_max = global_hit.as_ref().map_or(f32::INFINITY, |hit| hit.hit.t);
        let tree_hit = self.first_hit_impl(objects, ray, &filter, 0, t_max);

        ObjectHit::closest_option(global_hit, tree_hit)
    }
}

impl AxisBox {
    pub fn intersects(self, ray: &Ray) -> Option<f32> {
        let mut t_min = f32::NEG_INFINITY;
        let mut t_max = f32::INFINITY;

        for axis in Axis3::ALL {
            let t1 = (self.low.get(axis) - ray.start.get(axis)) / ray.direction.get(axis);
            let t2 = (self.high.get(axis) - ray.start.get(axis)) / ray.direction.get(axis);
            t_min = t_min.max(t1.min(t2));
            t_max = t_max.min(t1.max(t2));
        }

        if t_max >= t_min && t_max > 0.0 { Some(t_min) } else { None }
    }
}

struct Builder<'a> {
    strategy: BVHSplitStrategy,
    objects: &'a [Object],
    ids: Vec<SmallId>,
    nodes: Vec<Node>,
}

impl Builder<'_> {
    fn get_object(&self, index: u32) -> &Object {
        &self.objects[self.ids[index as usize].index as usize]
    }

    fn compute_bound(&self, start: u32, len: NonZeroU32) -> AxisBox {
        (start..(start + len.get()))
            .map(|index| AxisBox::for_object(self.get_object(index)))
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
        let (split_axis, split_value) = match self.find_best_split(start, len, bound) {
            Some(split) => split,
            None => return,
        };

        // rearrange the objects
        let split_index = partition(
            &mut self.ids[start as usize..][..len.get() as usize],
            |&id| object_centroid(&self.objects[id.index as usize]).get(split_axis) < split_value,
        ) as u32;

        // stop if one of the children is empty
        let left_len = NonZeroU32::new(split_index);
        let right_len = NonZeroU32::new(len.get() - split_index);

        let (left_len, right_len) = match (left_len, right_len) {
            (Some(left_len), Some(right_len)) => (left_len, right_len),
            (None, _) | (_, None) => return,
        };

        // push the children
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

    fn find_best_split(&self, start: u32, len: NonZeroU32, bound: AxisBox) -> Option<(Axis3, f32)> {
        match self.strategy {
            BVHSplitStrategy::SplitLargestAxis =>
                self.find_best_split_largest_axis(bound),
            BVHSplitStrategy::SurfaceAreaHeuristic { test_planes } =>
                self.find_best_split_surface_area(start, len, bound, test_planes),
        }
    }

    fn find_best_split_largest_axis(&self, bound: AxisBox) -> Option<(Axis3, f32)> {
        let extend = bound.high - bound.low;
        let split_axis = Axis3::ALL.into_iter()
            .max_by_key(|&a| Total::from_inner(extend.get(a)))
            .unwrap();
        let split_value = (bound.low.get(split_axis) + bound.high.get(split_axis)) / 2.0;
        Some((split_axis, split_value))
    }

    fn find_best_split_surface_area(&self, start: u32, len: NonZeroU32, bound: AxisBox, test_planes: Option<usize>) -> Option<(Axis3, f32)> {
        // no point even trying to split if we don't have enough nodes
        if len.get() < 2 {
            return None;
        }

        let mut best = None;
        let mut best_cost = f32::INFINITY;

        let mut try_split = |axis: Axis3, value: f32| {
            let cost = self.eval_potential_split(start, len, axis, value);
            if cost <= best_cost {
                best = Some((axis, value));
                best_cost = cost;
            }
        };

        if let Some(test_planes) = test_planes.filter(|&test_planes| len.get() as usize > test_planes) {
            // try a fixed number of test planes
            for pi in 0..test_planes {
                for axis in Axis3::ALL {
                    let value = lerp((pi + 1) as f32 / test_planes as f32, bound.low.get(axis), bound.high.get(axis));
                    try_split(axis, value)
                }
            }
        } else {
            // try each object centroid
            for index in start..(start + len.get()) {
                let centroid = object_centroid(self.get_object(index));
                for axis in Axis3::ALL {
                    let value = centroid.get(axis);
                    try_split(axis, value);
                }
            }
        }

        let curr_cost = len.get() as f32 * bound.area();
        if best_cost < curr_cost {
            best
        } else {
            None
        }
    }

    /// Evaluate the surface area heuristic for the given potential split.
    fn eval_potential_split(&self, start: u32, len: NonZeroU32, axis: Axis3, value: f32) -> f32 {
        const INF: f32 = f32::INFINITY;

        let mut left_count: u32 = 0;
        let mut left_low = Point3::new(INF, INF, INF);
        let mut left_high = Point3::new(-INF, -INF, -INF);
        let mut right_count: u32 = 0;
        let mut right_low = Point3::new(INF, INF, INF);
        let mut right_high = Point3::new(-INF, -INF, -INF);

        for index in start..(start + len.get()) {
            let centroid = object_centroid(self.get_object(index));
            if centroid.get(axis) < value {
                left_low = left_low.min(centroid);
                left_high = left_high.max(centroid);
                left_count += 1;
            } else {
                right_low = right_low.min(centroid);
                right_high = right_high.max(centroid);
                right_count += 1;
            }
        }

        if left_count == 0 || right_count == 0 {
            return f32::INFINITY;
        }

        let left_box = AxisBox::new(left_low, left_high);
        let right_box = AxisBox::new(right_low, right_high);

        left_count as f32 * left_box.area() + right_count as f32 * right_box.area()
    }

    fn check(&self, global_ids: &[SmallId]) {
        let mut seen = vec![false; self.objects.len()];

        self.check_node(0, &mut seen);

        for &id in global_ids {
            let flag = &mut seen[id.index as usize];
            assert!(!*flag);
            *flag = true;
        }

        assert!(seen.iter().all(|&b| b));
    }

    fn check_node(&self, node: u32, seen: &mut [bool]) -> AxisBox {
        let node = &self.nodes[node as usize];
        let actual_bound = match node.kind {
            NodeKind::Leaf { start, len } => {
                (start..start + len.get()).map(|index| {
                    let flag = &mut seen[self.ids[index as usize].index as usize];
                    assert!(!*flag);
                    *flag = true;

                    AxisBox::for_object(self.get_object(index))
                })
                    .reduce(AxisBox::combine).unwrap()
            }
            NodeKind::Branch { left_index } => {
                let bound_left = self.check_node(left_index, seen);
                let bound_right = self.check_node(left_index + 1, seen);
                bound_left.combine(bound_right)
            }
        };

        assert_eq!(node.bound, actual_bound);
        actual_bound
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
        write!(f, "BVH(global={}, ids={}, nodes={})", self.global_ids.len(), self.ids.len(), self.nodes.len())
    }
}

impl SmallId {
    fn to_large(self) -> ObjectId {
        ObjectId { index: self.index as usize }
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
        assert!(aabb.intersects(&ray).is_some());
    }
}
