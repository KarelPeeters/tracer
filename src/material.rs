use rand::distributions::Bernoulli;

use crate::Color;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Material {
    Fixed { color: Color },
    Mixed { color: Color, diff_prob: Bernoulli },
}