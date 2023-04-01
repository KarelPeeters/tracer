use std::fs::read_to_string;
use std::marker::PhantomData;
use std::path::Path;

use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;
use rand_distr::Distribution;
use rand_distr::UnitSphere;
use wavefront_obj::obj;

use crate::common::math::{Angle, Point3, Transform, Unit, Vec3};
use crate::common::scene::{Camera, Color, Material, MaterialType, Medium, Object, Scene, Shape};
use crate::common::util::{obj_to_triangles, triangle_as_transform};

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

pub fn material_fixed(color: Color, camera_only: bool) -> Material {
    Material {
        material_type: MaterialType::Fixed { camera_only },
        albedo: color,
        emission: BLACK,
        inside: VACUUM,
        outside: VACUUM,
    }
}

/// A cuboid centered around the origin with edge lengths given by `size`.
pub fn objects_cuboid(material: Material, size: Vec3, transform: Transform) -> Vec<Object> {
    let cx = size.x / 2.0;
    let cy = size.y / 2.0;
    let cz = size.z / 2.0;

    let points = [
        Point3::new(-cx, -cy, -cz),
        Point3::new(-cx, -cy, cz),
        Point3::new(-cx, cy, -cz),
        Point3::new(-cx, cy, cz),
        Point3::new(cx, -cy, -cz),
        Point3::new(cx, -cy, cz),
        Point3::new(cx, cy, -cz),
        Point3::new(cx, cy, cz),
    ];

    let triangles = [
        (0, 1, 2),
        (1, 2, 3),
        (4, 5, 6),
        (5, 6, 7),
        (0, 1, 4),
        (1, 4, 5),
        (2, 3, 6),
        (3, 6, 7),
        (0, 2, 4),
        (2, 4, 6),
        (1, 3, 5),
        (3, 5, 7),
    ];

    triangles.into_iter().map(|(a, b, c)| {
        Object {
            shape: Shape::Triangle,
            material,
            transform: transform * triangle_as_transform(points[a], points[b], points[c]),
        }
    }).collect()
}

pub fn objects_axes(brightness: f32, radius_axis: f32, radius_dot: Option<f32>, cube_dots: bool) -> Vec<Object> {
    let scale_axis = Transform::scale(radius_axis);
    let material_x = material_fixed(Color::new(brightness, 0.0, 0.0), true);
    let material_y = material_fixed(Color::new(0.0, brightness, 0.0), true);
    let material_z = material_fixed(Color::new(0.0, 0.0, brightness), true);
    let material_cube = material_fixed(BLACK, true);

    let mut result = vec![];

    result.push(Object {
        shape: Shape::Cylinder,
        material: material_x,
        transform: Transform::rotate(Vec3::z_axis(), Angle::degrees(90.0)) * scale_axis,
    });
    result.push(Object {
        shape: Shape::Cylinder,
        material: material_y,
        transform: scale_axis,
    });
    result.push(Object {
        shape: Shape::Cylinder,
        material: material_z,
        transform: Transform::rotate(Vec3::x_axis(), Angle::degrees(90.0)) * scale_axis,
    });

    if let Some(radius_dot) = radius_dot {
        let scale_dot = Transform::scale(radius_dot);
        result.push(Object {
            shape: Shape::Sphere,
            material: material_x,
            transform: Transform::translate(Vec3::new(1.0, 0.0, 0.0)) * scale_dot,
        });
        result.push(Object {
            shape: Shape::Sphere,
            material: material_y,
            transform: Transform::translate(Vec3::new(0.0, 1.0, 0.0)) * scale_dot,
        });
        result.push(Object {
            shape: Shape::Sphere,
            material: material_z,
            transform: Transform::translate(Vec3::new(0.0, 0.0, 1.0)) * scale_dot,
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
                    transform: Transform::translate(coord) * scale_dot,
                });
            }
        }
    }

    result
}

pub fn scene_single_red_sphere() -> Scene {
    Scene {
        objects: vec![
            Object {
                shape: Shape::Plane,
                material: material_diffuse(color_by_name("grey")),
                transform: Transform::rotate(Vec3::x_axis(), Angle::degrees(90.0)),
            },
            Object {
                shape: Shape::Sphere,
                material: material_glass(Color::new(1.0, 0.1, 0.1)),
                transform: Transform::translate(Vec3::new(0.0, 1.0, 0.0)),
            },
            Object {
                shape: Shape::Sphere,
                material: material_light(Color::new(1.0, 1.0, 1.0) * 1_000.0),
                transform: Transform::translate(Vec3::new(10.0, 10.0, -5.0)),
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

pub fn scene_colored_spheres() -> Scene {
    Scene {
        objects: vec![
            //light
            Object {
                shape: Shape::Sphere,
                material: material_light(Color::new(1.0, 1.0, 1.0) * 500.0),
                transform: Transform::scale(3.0) * Transform::translate(Vec3::new(10.0, 20.0, -10.0)),
            },
            //floor
            Object {
                shape: Shape::Plane,
                material: material_diffuse(Color::new(0.9, 0.9, 0.9)),
                transform: Transform::rotate(Vec3::x_axis(), Angle::degrees(90.0)),
            },
            //spheres
            Object {
                shape: Shape::Sphere,
                material: material_mixed(Color::new(1.0, 0.05, 0.05), 0.5),
                transform: Transform::translate(Vec3::new(-3.0, 1.0, -5.0)),
            },
            Object {
                shape: Shape::Sphere,
                material: material_glass(Color::new(0.4, 0.4, 1.0)),
                transform: Transform::translate(Vec3::new(0.0, 1.0, -5.0)),
            },
            Object {
                shape: Shape::Sphere,
                material: material_mixed(Color::new(0.05, 1.0, 0.05), 0.5),
                transform: Transform::translate(Vec3::new(3.0, 1.0, -5.0)),
            },
        ],
        sky_emission: color_gray(0.1),
        camera: Camera {
            fov_horizontal: Angle::degrees(90.0),
            transform: Transform::look_at(
                Point3::new(0.0, 1.5, 5.0),
                Point3::new(0.0, 1.0, -5.0),
                Vec3::y_axis(),
            ),
            medium: VACUUM,
        },
    }
}

pub fn scene_obj_file(path: impl AsRef<Path>, transform: Transform) -> Scene {
    let mut objects = vec![
        // floor
        Object {
            shape: Shape::Plane,
            material: material_diffuse(color_by_name("grey")),
            transform: Transform::rotate(Vec3::x_axis(), Angle::degrees(90.0)),
        },
        // light
        Object {
            shape: Shape::Sphere,
            material: material_light(WHITE * 1000.0),
            transform: Transform::scale(3.0) * Transform::translate(Vec3::new(10.0, 20.0, 10.0)),
        },
    ];

    let obj_string = read_to_string(path)
        .expect("Failed to read obj file");
    let object_set = obj::parse(obj_string)
        .expect("Error while parsing obj file");
    let cube = object_set.objects.first()
        .expect("No object found");

    let material_cube = material_diffuse(color_by_name("grey"));
    objects.extend(obj_to_triangles(cube, material_cube, transform));

    Scene {
        objects,
        sky_emission: color_by_name("gray"),
        camera: Camera {
            fov_horizontal: Angle::degrees(90.0),
            transform: Transform::look_at(
                Point3::new(0.0, 1.5, 3.0),
                Point3::new(0.0, 1.0, 0.0),
                Vec3::y_axis(),
            ),
            medium: VACUUM,
        },
    }
}

pub fn scene_random_tiles() -> Scene {
    let mut objects = vec![];
    let rng = &mut SmallRng::seed_from_u64(0);

    objects.push(Object {
        shape: Shape::Sphere,
        material: material_light(WHITE * 10000.0),
        transform: Transform::translate(Vec3::new(0.0, 0.0, 100.0)),
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

        let transform = Transform::translate(trans) * Transform::rotate(rot_axis, rot_angle) * Transform::scale(scale);

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

pub fn scene_cornell_box() -> Scene {
    let mut objects = vec![];

    // TODO add temperature -> rgb utility function
    let light_color = Color::new(255.0, 254.0, 250.0) / 255.0; // 6500K

    let wall_size = Vec3::new(0.550, 0.5488, 0.5592);

    // sphere light
    {
        let w = 0.1;
        let h = 0.01;

        let r = (4.0 * h * h + w * w) / (8.0 * h);
        let y = (w * w - 4.0 * h * h) / (8.0 * h);

        println!("Sphere light with r={r}, y={y}");
        objects.push(Object {
            shape: Shape::Sphere,
            material: material_light(light_color * 100.0),
            transform: Transform::translate(Vec3::new(wall_size.x / 2.0, wall_size.y + y, wall_size.z / 2.0)) * Transform::scale(r),
        });
    }

    let mut push_triangle = |a: Point3, b: Point3, c: Point3, material: Material| {
        let transform = triangle_as_transform(a, b, c);
        let object = Object { shape: Shape::Triangle, material, transform };
        objects.push(object);
    };

    // square light
    {
        // let size = 0.1;
        // let eps = 0.001;

        // let center = Point3::new(0.25, 1.0 - eps, 0.25);
        // let dx = Vec3::new(size / 2.0, 0.0, 0.0);
        // let dz = Vec3::new(0.0, 0.0, size / 2.0);

        // let material = material_light(light_color * 10.0);

        // TODO support triangle lights
        // push_triangle(center - dx - dz, center - dx + dz, center + dx + dz, material);
        // push_triangle(center - dx - dz, center + dx + dz, center + dx - dz, material);
    }

    // walls
    {
        let corners = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(wall_size.x, 0.0, 0.0),
            Point3::new(wall_size.x, wall_size.y, 0.0),
            Point3::new(0.0, wall_size.y, 0.0),
            Point3::new(0.0, 0.0, wall_size.z),
            Point3::new(wall_size.x, 0.0, wall_size.z),
            Point3::new(wall_size.x, wall_size.y, wall_size.z),
            Point3::new(0.0, wall_size.y, wall_size.z),
        ];

        let mut push_int_triangle = |a: usize, b: usize, c: usize, material: Material| {
            push_triangle(corners[a], corners[b], corners[c], material);
        };

        let wall_gray = material_diffuse(color_gray(0.4));
        let wall_green = material_diffuse(Color::new(0.5, 0.0, 0.0));
        let wall_red = material_diffuse(Color::new(0.0, 0.5, 0.0));

        //  top
        push_int_triangle(3, 2, 6, wall_gray);
        push_int_triangle(3, 6, 7, wall_gray);
        // bottom
        push_int_triangle(0, 1, 5, wall_gray);
        push_int_triangle(0, 5, 4, wall_gray);
        // back
        push_int_triangle(0, 1, 2, wall_gray);
        push_int_triangle(0, 2, 3, wall_gray);
        // left
        push_int_triangle(0, 3, 7, wall_green);
        push_int_triangle(0, 7, 4, wall_green);
        // right
        push_int_triangle(1, 2, 6, wall_red);
        push_int_triangle(1, 6, 5, wall_red);
    }

    // boxes
    let material_box = material_diffuse(color_gray(0.5));
    objects.extend(objects_cuboid(
        material_box,
        Vec3::new(0.165, 0.165, 0.165),
        Transform::translate(Vec3::new(0.37035, 0.165 / 2.0, 0.38669)) * Transform::rotate(Vec3::y_axis(), Angle::degrees(-106.0)),
    ));
    objects.extend(objects_cuboid(
        material_box,
        Vec3::new(0.165, 0.33, 0.165),
        Transform::translate(Vec3::new(0.18489, 0.33 / 2.0, 0.2072)) * Transform::rotate(Vec3::y_axis(), Angle::degrees(-162.0)),
    ));

    Scene {
        objects,
        sky_emission: BLACK,
        camera: Camera {
            fov_horizontal: Angle::degrees(36.0),
            transform: Transform::look_in_dir(Point3::new(wall_size.x / 2.0, wall_size.y / 2.0, 1.35), -Vec3::z_axis(), Vec3::y_axis()),
            medium: VACUUM,
        },
    }
}