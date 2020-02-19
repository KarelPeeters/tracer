use crate::Color;

#[derive(Debug)]
pub enum Material {
    Fixed(Color),
    Mirror,
}