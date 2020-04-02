pub type Vec3 = nalgebra::Vector3<f32>;
pub type Point3 = nalgebra::Point3<f32>;
pub type Color = palette::LinSrgb;
pub type Transform = nalgebra::Affine3<f32>;

#[derive(Debug, Copy, Clone)]
pub enum Shape {
    //unit sphere with center at origin
    Sphere,
    //vertical plane including xy axes
    Plane,
    //triangle with corners at (0,0,0), (1,0,0) and (0,1,0)
    Triangle,
}

#[derive(Copy, Clone, Debug)]
pub struct Material {
    pub emission: Color,
    pub albedo: Color,
    pub diffuse: bool,
}

#[derive(Debug)]
pub struct Object {
    pub shape: Shape,
    pub material: Material,

    pub transform: Transform,
}

//perspective camera at origin with X to the right and Y upwards looking towards negative Z
#[derive(Debug)]
pub struct Camera {
    pub fov_horizontal: f32,
    pub transform: Transform,
}

#[derive(Debug)]
pub struct Scene {
    pub objects: Vec<Object>,
    pub sky_emission: Color,
    pub camera: Camera,
}
