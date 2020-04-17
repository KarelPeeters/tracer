use std::ops::Deref;

use alga::general::SubsetOf;
use nalgebra::convert;

pub type Vec3 = nalgebra::Vector3<f32>;
pub type Vec2 = nalgebra::Vector2<f32>;
pub type Point3 = nalgebra::Point3<f32>;
pub type Color = palette::LinSrgb;
pub type Transform = nalgebra::Affine3<f32>;

#[derive(Debug)]
pub struct BiTransform {
    fwd: Transform,
    inv: Transform,
}

impl<W: SubsetOf<Transform>> From<W> for BiTransform {
    fn from(transform: W) -> Self {
        let affine = convert(transform);
        Self {
            fwd: affine,
            inv: affine.inverse(),
        }
    }
}

impl BiTransform {
    pub fn inv(&self) -> BiTransform {
        Self {
            fwd: self.inv.clone(),
            inv: self.fwd.clone(),
        }
    }
}

impl Deref for BiTransform {
    type Target = Transform;

    fn deref(&self) -> &Self::Target {
        &self.fwd
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Shape {
    //unit sphere with center at origin
    Sphere,
    //vertical plane including xy axes
    Plane,
    //triangle with corners at (0,0,0), (1,0,0) and (0,1,0)
    Triangle,
    //cylinder with radius 1 around the y-axis
    Cylinder,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MaterialType {
    Diffuse, Mirror, Transparent,
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

    pub transform: BiTransform,
}

//perspective camera at origin with X to the right and Y upwards looking towards negative Z
#[derive(Debug)]
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
