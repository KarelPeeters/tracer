use nalgebra::{Rotation3, Unit};

use crate::geometry::Ray;
use crate::tracer::{Point3, Vec3};

pub trait Camera {
    /// Cast a ray at pixel position (xi, yi) on a screen of pixel size (width, height)
    fn ray(&self, width: f32, height: f32, xi: f32, yi: f32) -> Ray;
}

#[derive(Debug)]
pub struct PerspectiveCamera {
    pub position: Point3,
    pub direction: Unit<Vec3>,
    pub fov_vertical: f32,
    pub fov_horizontal: f32,
}

impl Camera for PerspectiveCamera {
    //TODO fix distortion on the top and bottom of the image and
    //     this also doesn't work for near-vertical camera directions yet
    fn ray(&self, width: f32, height: f32, xi: f32, yi: f32) -> Ray {
        let pitch = self.fov_vertical * (yi / height - 0.5);
        let yaw = self.fov_horizontal * (xi / width - 0.5);
        let rot = Rotation3::from_euler_angles(
            pitch,
            yaw,
            0.0,
        );

        Ray {
            start: self.position.clone(),
            direction: rot * &self.direction,
        }
    }
}

pub struct OrthographicCamera {
    pub position: Point3,
    pub direction: Unit<Vec3>,
    pub width: f32,
}

impl Camera for OrthographicCamera {
    fn ray(&self, width: f32, height: f32, xi: f32, yi: f32) -> Ray {
        let self_height = self.width / width * height;
        let delta = Vec3::new((xi / width - 0.5) * self.width, -(yi / height - 0.5) * self_height, 0.0);

        Ray {
            start: &self.position + &delta,
            direction: self.direction.clone(),
        }
    }
}