use std::fs::read_to_string;
use std::marker::PhantomData;

use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use rand_distr::Distribution;
use rand_distr::UnitSphere;
use wavefront_obj::obj;

use crate::common::math::{Angle, Point3, Transform, Unit, Vec3};
use crate::common::scene::{Camera, Color, Material, MaterialType, Medium, Object, Scene, Shape};
use crate::common::util::obj_to_triangles;

pub const VACUUM_IOR: f32 = 1.0;
pub const GLASS_IOR: f32 = 1.52;

pub const BLACK: Color = Color { red: 0.0, green: 0.0, blue: 0.0, standard: PhantomData };
pub const WHITE: Color = Color { red: 1.0, green: 1.0, blue: 1.0, standard: PhantomData };

pub const VACUUM: Medium = Medium { index_of_refraction: 1.0, volumetric_color: WHITE };

pub fn color_by_name(name: &str) -> Color {
    palette::Srgb::from_format(palette::named::from_str(name).expect("Invalid color name"))
        .into_linear()
}

pub fn color_gray(v: f32) -> Color {
    Color::new(v, v, v)
}

pub fn medium_glass(volumetric_color: Color) -> Medium {
    Medium {
        index_of_refraction: GLASS_IOR,
        volumetric_color,
    }
}

pub fn material_diffuse(albedo: Color) -> Material {
    Material {
        material_type: MaterialType::Diffuse,

        albedo,
        emission: BLACK,

        inside: VACUUM,
        outside: VACUUM,
    }
}

pub fn material_mixed(albedo: Color, diffuse_fraction: f32) -> Material {
    assert!((0.0..=1.0).contains(&diffuse_fraction));
    Material {
        material_type: MaterialType::DiffuseMirror(diffuse_fraction),
        albedo,
        emission: BLACK,
        inside: VACUUM,
        outside: VACUUM,
    }
}

pub fn material_glass(volumetric_color: Color) -> Material {
    Material {
        material_type: MaterialType::Transparent,
        albedo: WHITE,
        emission: BLACK,
        inside: medium_glass(volumetric_color),
        outside: VACUUM,
    }
}

pub fn material_light(emission: Color) -> Material {
    Material {
        material_type: MaterialType::Diffuse,
        albedo: BLACK,
        emission,
        inside: VACUUM,
        outside: VACUUM,
    }
}

pub fn material_fixed(color: Color) -> Material {
    Material {
        material_type: MaterialType::Fixed,
        albedo: color,
        emission: BLACK,
        inside: VACUUM,
        outside: VACUUM,
    }
}

pub fn axes(brightness: f32, radius_axis: f32, radius_dot: Option<f32>, cube_dots: bool) -> Vec<Object> {
    let scale_axis = Transform::scaling(radius_axis);
    let material_x = material_fixed(Color::new(brightness, 0.0, 0.0));
    let material_y = material_fixed(Color::new(0.0, brightness, 0.0));
    let material_z = material_fixed(Color::new(0.0, 0.0, brightness));
    let material_cube = material_fixed(BLACK);

    let mut result = vec![];

    result.push(Object {
        shape: Shape::Cylinder,
        material: material_x,
        transform: Transform::rotation(Vec3::z_axis(), Angle::degrees(90.0)) * scale_axis,
    });
    result.push(Object {
        shape: Shape::Cylinder,
        material: material_y,
        transform: scale_axis,
    });
    result.push(Object {
        shape: Shape::Cylinder,
        material: material_z,
        transform: Transform::rotation(Vec3::x_axis(), Angle::degrees(90.0)) * scale_axis,
    });

    if let Some(radius_dot) = radius_dot {
        let scale_dot = Transform::scaling(radius_dot);
        result.push(Object {
            shape: Shape::Sphere,
            material: material_x,
            transform: Transform::translation(Vec3::new(1.0, 0.0, 0.0)) * scale_dot,
        });
        result.push(Object {
            shape: Shape::Sphere,
            material: material_y,
            transform: Transform::translation(Vec3::new(0.0, 1.0, 0.0)) * scale_dot,
        });
        result.push(Object {
            shape: Shape::Sphere,
            material: material_z,
            transform: Transform::translation(Vec3::new(0.0, 0.0, 1.0)) * scale_dot,
        });

        if cube_dots {
            let coords = [
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(0.0, 1.0, 1.0),
                Vec3::new(1.0, 0.0, 1.0),
                Vec3::new(1.0, 1.0, 0.0),
            ];
            for coord in coords {
                result.push(Object {
                    shape: Shape::Sphere,
                    material: material_cube,
                    transform: Transform::translation(coord) * scale_dot,
                });
            }
        }
    }

    result
}

pub fn single_red_sphere() -> Scene {
    Scene {
        objects: vec![
            Object {
                shape: Shape::Plane,
                material: material_diffuse(color_by_name("grey")),
                transform: Transform::rotation(Vec3::x_axis(), Angle::degrees(90.0)),
            },
            Object {
                shape: Shape::Sphere,
                material: material_glass(Color::new(1.0, 0.1, 0.1)),
                transform: Transform::translation(Vec3::new(0.0, 1.0, 0.0)),
            },
            Object {
                shape: Shape::Sphere,
                material: material_light(Color::new(1.0, 1.0, 1.0) * 1_000.0),
                transform: Transform::translation(Vec3::new(10.0, 10.0, -5.0)),
            },
        ],
        sky_emission: color_by_name("gray"),
        camera: Camera {
            fov_horizontal: Angle::degrees(90.0),
            transform: Transform::look_at(
                Point3::new(0.0, 1.5, 5.0),
                Point3::new(0.0, 1.0, 0.0),
                Vec3::y_axis(),
            ),
            medium: VACUUM,
        },
    }
}

pub fn colored_spheres() -> Scene {
    Scene {
        objects: vec![
            //light
            Object {
                shape: Shape::Sphere,
                material: material_light(Color::new(1.0, 1.0, 1.0) * 500.0),
                transform: Transform::scaling(3.0) * Transform::translation(Vec3::new(10.0, 20.0, -10.0)),
            },
            //floor
            Object {
                shape: Shape::Plane,
                material: material_diffuse(Color::new(0.9, 0.9, 0.9)),
                transform: Transform::rotation(Vec3::x_axis(), Angle::degrees(90.0)),
            },
            //spheres
            Object {
                shape: Shape::Sphere,
                material: material_mixed(Color::new(1.0, 0.05, 0.05), 0.5),
                transform: Transform::translation(Vec3::new(-3.0, 1.0, -5.0)),
            },
            Object {
                shape: Shape::Sphere,
                material: material_glass(Color::new(0.4, 0.4, 1.0)),
                transform: Transform::translation(Vec3::new(0.0, 1.0, -5.0)),
            },
            Object {
                shape: Shape::Sphere,
                material: material_mixed(Color::new(0.05, 1.0, 0.05), 0.5),
                transform: Transform::translation(Vec3::new(3.0, 1.0, -5.0)),
            },
        ],
        sky_emission: color_gray(0.1),
        camera: Camera {
            fov_horizontal: Angle::degrees(90.0),
            transform: Transform::look_at(
                Point3::new(0.0, 1.5, 1.0),
                Point3::new(0.0, 1.0, -5.0),
                Vec3::y_axis(),
            ),
            medium: VACUUM,
        },
    }
}

pub fn cube() -> Scene {
    let mut objects = vec![
        //floor
        Object {
            shape: Shape::Plane,
            material: material_diffuse(color_by_name("grey")),
            transform: Transform::rotation(Vec3::x_axis(), Angle::degrees(90.0)),
        },
    ];

    let obj_string = read_to_string("ignored/models/cube.obj")
        .expect("Failed to read obj file");
    let object_set = obj::parse(obj_string)
        .expect("Error while parsing obj file");
    let cube = object_set.objects.first()
        .expect("No object found");

    let material_cube = material_diffuse(color_by_name("grey"));
    objects.extend(obj_to_triangles(cube, material_cube, Default::default()));

    Scene {
        objects,
        sky_emission: color_by_name("gray"),
        camera: Camera {
            fov_horizontal: Angle::degrees(90.0),
            transform: Transform::look_at(
                Point3::new(0.0, 1.5, 5.0),
                Point3::new(0.0, 1.0, 0.0),
                Vec3::y_axis(),
            ),
            medium: VACUUM,
        },
    }
}

pub fn random_tiles() -> Scene {
    let mut objects = vec![];
    let rng = &mut SmallRng::seed_from_u64(0);

    objects.push(Object {
        shape: Shape::Sphere,
        material: material_light(WHITE * 10000.0),
        transform: Transform::translation(Vec3::new(0.0, 0.0, 100.0)),
    });

    for _ in 0..100_000 {
        let trans = Vec3::new(
            rng.gen_range(-100.0..100.0),
            rng.gen_range(-100.0..100.0),
            rng.gen_range(-20.0..20.0),
        );

        let rot_axis = Unit::new_unchecked(Vec3::from_slice(&UnitSphere.sample(rng)));
        let rot_angle = Angle::degrees(rng.gen_range(0.0..360.0));

        let scale = rng.gen_range(0.5..2.0);

        let transform = Transform::translation(trans) * Transform::rotation(rot_axis, rot_angle) * Transform::scaling(scale);

        objects.push(Object {
            shape: Shape::Square,
            material: material_diffuse(WHITE),
            transform,
        });
    }

    Scene {
        objects,
        sky_emission: color_gray(0.01),
        camera: Camera {
            fov_horizontal: Angle::degrees(90.0),
            transform: Transform::look_at(
                Point3::new(0.0, -4.0, 40.0),
                Point3::origin(),
                Vec3::y_axis(),
            ),
            medium: VACUUM,
        },
    }
}
