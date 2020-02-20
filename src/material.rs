use crate::Color;

#[derive(Debug)]
pub enum Material {
    Fixed(Color),
    Diffuse(Color),
    Mirror,
}