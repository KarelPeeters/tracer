use std::cmp::max;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Range;

use decorum::N32;
use itertools::Itertools;

use crate::common::aabb::AxisBox;
use crate::common::math::{Axis3, Axis3Owner};
use crate::common::scene::Object;
use crate::cpu::accel::{Accel, first_hit, ObjectId};
use crate::cpu::geometry::{ObjectHit, Ray};

// TODO fix wrongly returned indices that cause light to be overactive
pub struct Octree {
    ids: Vec<ObjectId>,
    nodes: Vec<Node>,

    node_root: usize,
}

struct Builder<'a> {
    max_flat_size: usize,
    objects: &'a [Object],

    ids: Vec<ObjectId>,
    nodes: Vec<Node>,
}

// TODO add per-node AABB for quick check beforehand?
// TODO switch to BVH
#[derive(Debug)]
enum Node {
    Flat(Range<usize>),
    Split {
        axis: Axis3,
        value: f32,
        node_lower: usize,
        node_higher: usize,
    },
}

impl Octree {
    pub fn new(objects: &[Object], max_flat_size: usize) -> Self {
        let objects_len = objects.len();

        let mut builder = Builder {
            max_flat_size,
            objects,
            ids: vec![],
            nodes: vec![],
        };

        let ids = (0..objects_len).map(ObjectId::new).collect_vec();
        let node_root = builder.build_node(&ids);

        Octree {
            nodes: builder.nodes,
            ids: builder.ids,
            node_root,
        }
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

impl Accel for Octree {
    fn first_hit(&self, objects: &[Object], ray: &Ray) -> Option<ObjectHit> {
        self.nodes[self.node_root].first_hit(self, objects, ray, f32::INFINITY)
    }
}

impl Builder<'_> {
    fn build_flat_node(&mut self, ids: &[ObjectId]) -> usize {
        let start = self.ids.len();
        self.ids.extend(ids);
        let end = self.ids.len();
        let node = Node::Flat(start..end);
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    fn build_node(&mut self, ids: &[ObjectId]) -> usize {
        if ids.len() <= self.max_flat_size {
            return self.build_flat_node(ids);
        }

        let mut best_axis = None;
        let mut best_split = f32::NAN;
        let mut best_cost = usize::MAX;

        for axis in [Axis3::X, Axis3::Y, Axis3::Z] {
            if let Some((split, cost)) = self.best_axis_split(ids, axis) {
                if best_axis.is_none() || cost < best_cost {
                    best_split = split;
                    best_axis = Some(axis);
                    best_cost = cost;
                }
            }
        }

        if let Some(axis) = best_axis {
            let (lower, higher) = self.split_objects(ids, axis, best_split);
            let node = Node::Split {
                axis,
                value: best_split,
                node_lower: self.build_node(&lower),
                node_higher: self.build_node(&higher),
            };
            self.nodes.push(node);
            self.nodes.len() - 1
        } else {
            self.build_flat_node(ids)
        }
    }

    fn split_objects(&self, ids: &[ObjectId], axis: Axis3, split: f32) -> (Vec<ObjectId>, Vec<ObjectId>) {
        let mut lower = vec![];
        let mut higher = vec![];
        for &id in ids {
            let b = AxisBox::for_object(&self.objects[id.index]);
            if b.low.get(axis) <= split {
                lower.push(id);
            }
            if b.high.get(axis) >= split {
                higher.push(id);
            }
        }
        (lower, higher)
    }

    fn best_axis_split(&self, ids: &[ObjectId], axis: Axis3) -> Option<(f32, usize)> {
        // collect edges
        let mut edges = vec![];
        for &id in ids {
            let b = AxisBox::for_object(&self.objects[id.index]);
            edges.push(N32::from_inner(b.low.get(axis)));
            edges.push(N32::from_inner(b.high.get(axis)));
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
            for &id in ids {
                let b = AxisBox::for_object(&self.objects[id.index]);
                if b.low.get(axis) <= split {
                    lower_count += 1;
                }
                if b.high.get(axis) >= split {
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
        if best_cost == ids.len() {
            return None;
        }

        best_split.map(|split| (split, best_cost))
    }
}

impl Node {
    fn first_hit<'a>(&self, octree: &'a Octree, objects: &[Object], ray: &Ray, mut t_max: f32) -> Option<ObjectHit> {
        match self {
            Node::Flat(range) => {
                let objects = range.clone().map(|i| &objects[octree.ids[i].index]);
                first_hit(objects, ray).map(|(index, hit)| {
                    ObjectHit {
                        id: octree.ids[range.start + index],
                        hit,
                    }
                })
            }
            &Node::Split { axis, value, node_lower, node_higher } => {
                let start_in_lower = ray.start.get(axis) <= value;
                let end_in_lower = ray.at(t_max).get(axis) <= value;

                // compute start hit
                let start_node = if start_in_lower { node_lower } else { node_higher };
                let start_hit = octree.nodes[start_node].first_hit(octree, objects, ray, t_max);

                if let Some(hit) = start_hit.as_ref() {
                    t_max = f32::min(t_max, hit.hit.t);
                }

                // compute end hit if end is different from start
                if end_in_lower != start_in_lower {
                    let end_node = if end_in_lower { node_lower } else { node_higher };
                    let end_hit = octree.nodes[end_node].first_hit(octree, objects, ray, t_max);
                    ObjectHit::closest_option(start_hit, end_hit)
                } else {
                    start_hit
                }
            }
        }
    }
}

impl Display for Octree {
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

impl Debug for Octree {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let (len, depth) = self.len_depth();
        writeln!(f, "Octree(ids={}, len={}, depth={}, nodes={})", self.ids.len(), len, depth, self.nodes.len())
    }
}