use std::cmp::max;
use std::fmt::{Debug, Formatter};
use std::ops::Range;

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
    objects: Vec<Object>,
    indices: Vec<usize>,
    nodes: Vec<Node>,
    node_root: usize,
}

struct Builder {
    max_flat_size: usize,

    all_objects: Vec<Object>,
    all_indices: Vec<usize>,
    all_nodes: Vec<Node>,
}

// TODO add per-node AABB for quick check beforehand?
// TODO switch to BVH
#[derive(Debug)]
enum Node {
    Flat(Range<usize>),
    Split {
        axis: Axis,
        value: f32,
        node_lower: usize,
        node_higher: usize,
    },
}

#[derive(Debug, Copy, Clone)]
enum Axis {
    X,
    Y,
    Z,
}

impl Octree {
    pub fn new(objects: Vec<Object>, max_flat_size: usize) -> Self {
        let mut builder = Builder {
            max_flat_size,
            all_objects: objects,
            all_indices: vec![],
            all_nodes: vec![],
        };

        let indices = (0..builder.all_objects.len()).collect_vec();
        let node_root = builder.build_node(&indices);

        Octree {
            objects: builder.all_objects,
            nodes: builder.all_nodes,
            indices: builder.all_indices,
            node_root,
        }
    }

    pub fn first_hit(&self, ray: &Ray) -> Option<ObjectHit> {
        self.nodes[self.node_root].first_hit(self, ray, f32::INFINITY)
    }

    pub fn len_depth(&self) -> (usize, usize) {
        self.len_depth_node(self.node_root)
    }

    fn len_depth_node(&self, node: usize) -> (usize, usize) {
        match &self.nodes[node] {
            Node::Flat(range) => (range.len(), 0),
            &Node::Split { node_lower, node_higher, .. } => {
                let (lower_len, lower_depth) = self.len_depth_node(node_lower);
                let (higher_len, higher_depth) = self.len_depth_node(node_higher);
                (lower_len + higher_len, max(lower_depth, higher_depth) + 1)
            }
        }
    }
}

impl Builder {
    fn build_flat_node(&mut self, indices: &[usize]) -> usize {
        let start = self.all_indices.len();
        self.all_indices.extend(indices);
        let end = self.all_indices.len();
        let node = Node::Flat(start..end);
        self.all_nodes.push(node);
        self.all_nodes.len() - 1
    }

    fn build_node(&mut self, indices: &[usize]) -> usize {
        if indices.len() <= self.max_flat_size {
            return self.build_flat_node(indices);
        }

        let mut best_axis = None;
        let mut best_split = f32::NAN;
        let mut best_cost = usize::MAX;

        for axis in [Axis::X, Axis::Y, Axis::Z] {
            if let Some((split, cost)) = self.best_axis_split(indices, axis) {
                if best_axis.is_none() || cost < best_cost {
                    best_split = split;
                    best_axis = Some(axis);
                    best_cost = cost;
                }
            }
        }

        if let Some(axis) = best_axis {
            let (lower, higher) = self.split_objects(indices, axis, best_split);
            let node = Node::Split {
                axis,
                value: best_split,
                node_lower: self.build_node(&lower),
                node_higher: self.build_node(&higher),
            };
            self.all_nodes.push(node);
            self.all_nodes.len() - 1
        } else {
            self.build_flat_node(indices)
        }
    }

    fn split_objects(&self, indices: &[usize], axis: Axis, split: f32) -> (Vec<usize>, Vec<usize>) {
        let mut lower = vec![];
        let mut higher = vec![];
        for &index in indices {
            let b = object_aabb(&self.all_objects[index]);
            if axis.value(b.low) <= split {
                lower.push(index);
            }
            if axis.value(b.high) >= split {
                higher.push(index);
            }
        }
        (lower, higher)
    }

    fn best_axis_split(&self, indices: &[usize], axis: Axis) -> Option<(f32, usize)> {
        // collect edges
        let mut edges = vec![];
        for &i in indices {
            let b = object_aabb(&self.all_objects[i]);
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
            for &i in indices {
                let b = object_aabb(&self.all_objects[i]);
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
        if best_cost == indices.len() {
            return None;
        }

        best_split.map(|split| (split, best_cost))
    }
}

impl Node {
    fn first_hit<'a>(&self, octree: &'a Octree, ray: &Ray, mut t_max: f32) -> Option<ObjectHit<'a>> {
        match self {
            Node::Flat(range) => {
                let objects = range.clone().map(|i| &octree.objects[octree.indices[i]]);
                first_hit(objects, ray)
            }
            &Node::Split { axis, value, node_lower, node_higher } => {
                let start_in_lower = axis.value(ray.start) <= value;
                let end_in_lower = axis.value(ray.at(t_max)) <= value;

                // compute start hit
                let start_node = if start_in_lower { node_lower } else { node_higher };
                let start_hit = octree.nodes[start_node].first_hit(octree, ray, t_max);

                if let Some(hit) = start_hit.as_ref() {
                    t_max = f32::min(t_max, hit.hit.t);
                }

                // compute end hit if end is different from start
                if end_in_lower != start_in_lower {
                    let end_node = if end_in_lower { node_lower } else { node_higher };
                    let end_hit = octree.nodes[end_node].first_hit(octree, ray, t_max);
                    ObjectHit::closest_option(start_hit, end_hit)
                } else {
                    start_hit
                }
            }
        }
    }
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
        self.nodes[self.node_root].debug_fmt(f, self, 1)?;
        writeln!(f, "}}")?;
        Ok(())
    }
}

impl Node {
    fn debug_fmt(&self, f: &mut Formatter<'_>, octree: &Octree, indent: usize) -> std::fmt::Result {
        let indent_str = " ".repeat(4 * indent);
        match self {
            Node::Flat(objects) => {
                writeln!(f, "{indent_str}Node::Flat {objects:?},")?;
            }
            &Node::Split { axis, value, node_lower, node_higher } => {
                writeln!(f, "{indent_str}Node::Split({axis:?}, {value}) [")?;
                octree.nodes[node_lower].debug_fmt(f, octree, indent + 1)?;
                octree.nodes[node_higher].debug_fmt(f, octree, indent + 1)?;
                writeln!(f, "{indent_str}],")?;
            }
        }
        Ok(())
    }
}