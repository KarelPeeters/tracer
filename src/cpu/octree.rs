use std::cmp::max;
use std::fmt::{Debug, Formatter};

use decorum::N32;
use itertools::Itertools;

use crate::common::math::Point3;
use crate::common::scene::{Object, Shape};
use crate::cpu::geometry::{ObjectHit, Ray};
use crate::cpu::renderer::first_hit;

/// Axis-aligned bounding box.
#[derive(Debug, Copy, Clone)]
pub struct AABBox {
    pub low: Point3,
    pub high: Point3,
}

pub struct Octree {
    root: Node,
}

#[derive(Debug)]
enum Node {
    // TODO add per-node AABB for quick check beforehand?
    Flat(Vec<Object>),
    Split {
        axis: Axis,
        value: f32,
        lower: Box<Node>,
        higher: Box<Node>,
    },
}

#[derive(Debug, Copy, Clone)]
enum Axis {
    X,
    Y,
    Z,
}

impl Octree {
    pub fn new(objects: &[Object], max_flat_size: usize) -> Self {
        Octree {
            root: Node::build(objects, max_flat_size)
        }
    }

    pub fn first_hit(&self, ray: &Ray) -> Option<ObjectHit> {
        self.root.first_hit(ray, f32::INFINITY)
    }

    pub fn len(&self) -> usize {
        self.root.len()
    }
}

impl Node {
    fn build(objects: &[Object], max_flat_size: usize) -> Self {
        if objects.len() <= max_flat_size {
            return Node::Flat(objects.to_vec());
        }

        let mut best_split = f32::NAN;
        let mut best_axis = None;
        let mut best_cost = usize::MAX;

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            if let Some((split, cost)) = best_axis_split(&objects, axis) {
                if best_axis.is_none() || cost < best_cost {
                    best_split = split;
                    best_axis = Some(axis);
                    best_cost = cost;
                }
            }
        }

        if let Some(axis) = best_axis {
            let (lower, higher) = split_objects(&objects, axis, best_split);
            Node::Split {
                axis,
                value: best_split,
                lower: Box::new(Node::build(&lower, max_flat_size)),
                higher: Box::new(Node::build(&higher, max_flat_size)),
            }
        } else {
            Node::Flat(objects.to_vec())
        }
    }

    fn first_hit(&self, ray: &Ray, mut t_max: f32) -> Option<ObjectHit> {
        match self {
            Node::Flat(objects) => first_hit(objects, ray),
            &Node::Split { axis, value, ref lower, ref higher } => {
                let start_in_lower = axis.value(ray.start) <= value;
                let end_in_lower = axis.value(ray.at(t_max)) <= value;

                // compute start hit
                let start = if start_in_lower { lower } else { higher };
                let start_hit = start.first_hit(ray, t_max);

                if let Some(hit) = start_hit.as_ref() {
                    t_max = f32::min(t_max, hit.hit.t);
                }

                // compute end hit if end is different from start
                if end_in_lower != start_in_lower {
                    let end = if end_in_lower { lower } else { higher };
                    let end_hit = end.first_hit(ray, t_max);
                    ObjectHit::closest_option(start_hit, end_hit)
                } else {
                    start_hit
                }
            }
        }
    }

    fn len(&self) -> usize {
        match self {
            Node::Flat(objects) => objects.len(),
            Node::Split { lower, higher, .. } => lower.len() + higher.len(),
        }
    }
}

fn split_objects(objects: &[Object], axis: Axis, split: f32) -> (Vec<Object>, Vec<Object>) {
    let mut lower = vec![];
    let mut higher = vec![];
    for object in objects {
        let b = object_aabb(object);
        if axis.value(b.low) <= split {
            lower.push(object.clone());
        }
        if axis.value(b.high) >= split {
            higher.push(object.clone());
        }
    }
    (lower, higher)
}

fn best_axis_split(objects: &[Object], axis: Axis) -> Option<(f32, usize)> {
    // collect edges
    let mut edges = vec![];
    for object in objects {
        let b = object_aabb(object);
        edges.push(N32::from_inner(axis.value(b.low)));
        edges.push(N32::from_inner(axis.value(b.high)));
    }

    edges.sort_unstable();
    edges.dedup();

    // find best split
    let mut best_split = None;
    let mut best_cost = usize::MAX;

    let edges = edges.iter().copied().map(N32::into_inner);
    for (prev, next) in edges.clone().zip(edges.skip(1)) {
        let split = (prev + next) / 2.0;

        if best_split.is_none() {
            best_split = Some(split);
            continue;
        }

        let mut lower_count = 0;
        let mut higher_count = 0;
        for object in objects {
            let b = object_aabb(object);
            if axis.value(b.low) <= split {
                lower_count += 1;
            }
            if axis.value(b.high) >= split {
                higher_count += 1;
            }
        }

        let cost = max(lower_count, higher_count);
        if cost < best_cost {
            best_split = Some(split);
            best_cost = cost;
        }
    }

    // if we didn't manage to separate anything we can't count this as a split
    if best_cost == objects.len() {
        return None;
    }

    best_split.map(|split| (split, best_cost))
}

impl Axis {
    fn value(self, point: Point3) -> f32 {
        match self {
            Axis::X => point.x,
            Axis::Y => point.y,
            Axis::Z => point.z,
        }
    }
}

impl AABBox {
    pub fn new(low: Point3, high: Point3) -> Self {
        // TODO add eps padding in here automatically?
        let delta = high - low;
        assert!(delta.x >= 0.0 && delta.y >= 0.0 && delta.z >= 0.0);
        Self { low, high }
    }

    pub fn for_each_corner(self, mut f: impl FnMut(Point3)) {
        f(self.low);
        f(Point3::new(self.high.x, self.low.y, self.low.z));
        f(Point3::new(self.low.x, self.high.y, self.low.z));
        f(Point3::new(self.low.x, self.low.y, self.high.z));
        f(Point3::new(self.high.x, self.high.y, self.low.z));
        f(Point3::new(self.high.x, self.low.y, self.high.z));
        f(Point3::new(self.low.x, self.high.y, self.high.z));
        f(self.high);
    }
}

fn object_aabb(object: &Object) -> AABBox {
    const INF: f32 = f32::INFINITY;
    let mut low = Point3::new(INF, INF, INF);
    let mut high = Point3::new(-INF, -INF, -INF);

    shape_aabb(object.shape).for_each_corner(|p_orig| {
        let p_trans = object.transform * p_orig;
        low = low.min(p_trans);
        high = high.max(p_trans);
    });

    AABBox::new(low, high)
}

fn shape_aabb(shape: Shape) -> AABBox {
    const INF: f32 = f32::INFINITY;
    match shape {
        Shape::Sphere => AABBox::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0)),
        Shape::Plane => AABBox::new(Point3::new(-INF, -INF, 0.0), Point3::new(INF, INF, 0.0)),
        Shape::Triangle => AABBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        Shape::Square => AABBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
        Shape::Cylinder => AABBox::new(Point3::new(-1.0, -INF, -1.0), Point3::new(1.0, INF, 1.0)),
    }
}

impl Debug for Octree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Octree {{")?;
        self.root.debug_fmt(f, 1)?;
        writeln!(f, "}}")?;
        Ok(())
    }
}

impl Node {
    fn debug_fmt(&self, f: &mut Formatter<'_>, indent: usize) -> std::fmt::Result {
        let indent_str = " ".repeat(4 * indent);
        match self {
            Node::Flat(objects) => {
                let shapes = objects.iter().map(|o| o.shape).collect_vec();
                writeln!(f, "{indent_str}Node::Flat {shapes:?},")?;
            }
            Node::Split { axis, value, lower, higher } => {
                writeln!(f, "{indent_str}Node::Split({axis:?}, {value}) [")?;
                lower.debug_fmt(f, indent + 1)?;
                higher.debug_fmt(f, indent + 1)?;
                writeln!(f, "{indent_str}],")?;
            }
        }
        Ok(())
    }
}