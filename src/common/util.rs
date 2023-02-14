use wavefront_obj::obj;
use wavefront_obj::obj::Primitive;

use crate::common::math::{Point3, Transform, Vec3};
use crate::common::scene::{Material, Object, Shape};

fn vertex_to_point(vertex: &obj::Vertex) -> Point3 {
    Point3::new(vertex.x as f32, vertex.y as f32, vertex.z as f32)
}

pub fn triangle_as_transform(a: Point3, b: Point3, c: Point3) -> Transform {
    let shift = Transform::translation(*Vec3::z_axis());
    let axes_to_shift = Transform::map_axes_to(
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(0.0, 1.0, 1.0)
    );
    let origin = Point3::origin();
    let axes_to_target = Transform::map_axes_to(a - origin, b - origin, c - origin);

    axes_to_target * axes_to_shift.inv() * shift
}

pub fn obj_to_triangles(obj: &obj::Object, material: Material, transform: Transform) -> impl Iterator<Item=Object> + '_ {
    obj.geometry.iter().flat_map(move |geometry|
        geometry.shapes.iter().filter_map(move |shape| {
            match shape.primitive {
                Primitive::Point(_) => None,
                Primitive::Line(_, _) => None,
                Primitive::Triangle((avi, _, _), (bvi, ..), (cvi, ..)) => {
                    let a = vertex_to_point(&obj.vertices[avi]);
                    let b = vertex_to_point(&obj.vertices[bvi]);
                    let c = vertex_to_point(&obj.vertices[cvi]);

                    let local_transform = triangle_as_transform(a, b, c);

                    Some(Object {
                        shape: Shape::Triangle,
                        material,
                        transform: transform * local_transform,
                    })
                }
            }
        })
    )
}

#[cfg(windows)]
pub fn lower_process_priority() {
    unsafe {
        let curr_process = windows::Win32::System::Threading::GetCurrentProcess();
        let idle_priority = windows::Win32::System::Threading::IDLE_PRIORITY_CLASS;
        windows::Win32::System::Threading::SetPriorityClass(curr_process, idle_priority);
    }
}

#[cfg(not(windows))]
pub fn lower_process_priority() {}