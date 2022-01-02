use crate::common::math::{Transform, Angle};

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

#[derive(Copy, Clone, Debug)]
pub enum MaterialType {
    Fixed,
    Diffuse,
    Mirror,
    Transparent,
    // f is the fraction of light that's diffuse, 0 <= f <= 1
    //TODO maybe just remove Diffuse and Mirror and make a single Opque material? or even just have a single material
    DiffuseMirror(f32),
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
    pub scatter_average_dist: Option<f32>,
    pub scatter_g: f32,
}

#[derive(Debug)]
pub struct Object {
    pub shape: Shape,
    pub material: Material,

    /*TODO this is kind of sad: most objects need way less data than this complex transform:
        sphere: (cx, cy, cz, r)
        plane: (nx, ny, nz, d)
        triangle: (ax, ay, az, bx, by, bz, cx, cy, cz, nx, ny, nz)
        cylinder: (cx, cy, cz, dx, dy)
       compared to transform which has 2 * 4 * 4 = 32 floats!
     */
    pub transform: Transform,
}

#[derive(Debug)]
/// Perspective camera at origin with X to the right and Y upwards looking towards negative Z
pub struct Camera {
    pub fov_horizontal: Angle,
    pub transform: Transform,

    pub medium: Medium,
}

#[derive(Debug)]
pub struct Scene {
    pub objects: Vec<Object>,
    pub sky_emission: Color,
    pub camera: Camera,
}
