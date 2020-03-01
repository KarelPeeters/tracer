use rand::distributions::Bernoulli;

use crate::tracer::Color;

#[derive(Debug)]
pub struct Material {
    pub reflect_color: Color,
    pub emission: Color,
    pub diff_prob: Bernoulli,
    pub transparent_prob: Bernoulli,
}

impl Material {
    pub fn basic(color: Color, diff_prob: f64, transparent_prob: f64) -> Material {
        Material {
            reflect_color: color,
            emission: Color::new(0.0, 0.0, 0.0),
            diff_prob: Bernoulli::new(diff_prob).expect("probability should be in [0..1]"),
            transparent_prob: Bernoulli::new(transparent_prob).expect("probability should be in [0..1]"),
        }
    }
}