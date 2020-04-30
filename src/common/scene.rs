use crate::common::math::Transform;

pub type Color = palette::LinSrgb;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Shape {
    /// Unit sphere with center at origin
    Sphere,
    /// Vertical plane including xy axes
    Plane,
    /// Triangle with corners at (0,0,0), (1,0,0) and (0,1,0)
    Triangle,
    /// Cylinder with radius 1 around the y-axis
    Cylinder,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MaterialType {
    Fixed,
    Diffuse,
    Mirror,
    Transparent,
}

#[derive(Copy, Clone, Debug)]
pub struct Material {
    pub material_type: MaterialType,

    pub emission: Color,
    pub albedo: Color,

    pub inside: Medium,
    pub outside: Medium,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Medium {
    pub index_of_refraction: f32,
    pub volumetric_color: Color,
}

#[derive(Debug)]
pub struct Object {
    pub shape: Shape,
    pub material: Material,

    pub transform: Transform,
}

#[derive(Debug)]
/// Perspective camera at origin with X to the right and Y upwards looking towards negative Z
pub struct Camera {
    pub fov_horizontal: f32,
    pub transform: Transform,

    pub medium: Medium,
}

#[derive(Debug)]
pub struct Scene {
    pub objects: Vec<Object>,
    pub sky_emission: Color,
    pub camera: Camera,
}
