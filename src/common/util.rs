use wavefront_obj::obj;
use wavefront_obj::obj::Primitive;

use crate::common::math::{Point3, Transform};
use crate::common::scene::{Material, Object, Shape};

fn vertex_to_point(vertex: &obj::Vertex) -> Point3 {
    Point3::new(vertex.x as f32, vertex.y as f32, vertex.z as f32)
}

pub fn triangle_as_transform(a: Point3, b: Point3, c: Point3) -> Transform {
    println!("Triangle with points {:?}, {:?}, {:?}", a, b, c);

    let db = b - a;
    let dc = c - a;
    let _n = -db.cross(dc);

    todo!()

    /*let transform = Transform::from_matrix_unchecked(Matrix4::new(
        db.x, dc.x, n.x, a.x,
        db.y, dc.y, n.y, a.y,
        db.z, dc.z, n.z, a.z,
        0.0, 0.0, 0.0, 1.0,
    ));*/

    // transform
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