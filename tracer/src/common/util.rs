use wavefront_obj::obj;
use wavefront_obj::obj::Primitive;

use crate::common::math::{Norm, Point3, Transform, Vec3};
use crate::common::scene::{Material, Object, Shape};

fn vertex_to_point(vertex: &obj::Vertex) -> Point3 {
    Point3::new(vertex.x as f32, vertex.y as f32, vertex.z as f32)
}

pub fn triangle_as_transform(a: Point3, b: Point3, c: Point3) -> Transform {
    // Conventions:
    // * The source triangle is `Shape::Triangle`, the target triangle is `[a, b, c]`.
    // * We're looking for a transform `T` such that `p_target_space = T * p_source_space`.
    // Note:
    // If either the source or target triangle intersects the origin a rotation is not be enough to build the mapping.
    // As a workaround we shift both triangles away from the origin along their normal.
    // (idea from https://math.stackexchange.com/a/3999779/286346)

    let target_normal = (b - a).cross(c - a).normalized();

    let shift_source = Transform::translate(*Vec3::z_axis());
    let shift_target = Transform::translate(*target_normal);

    let axes_to_shifted_source = Transform::rotate_axes_to(
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 1.0),
        Vec3::new(0.0, 1.0, 1.0)
    );

    let axes_to_shifted_target = Transform::rotate_axes_to(
        a.coords() + *target_normal,
        b.coords() + *target_normal,
        c.coords() + *target_normal
    );

    shift_target.inv() * axes_to_shifted_target * axes_to_shifted_source.inv() * shift_source
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

#[cfg(test)]
mod test {
    use crate::common::math::Point3;
    use crate::common::util::triangle_as_transform;

    #[test]
    fn triangle_as_transform_including_origin() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(0.5, 0.0, 0.0);
        let c = Point3::new(0.5, 0.5, 0.0);

        let trans = triangle_as_transform(a, b, c);
        println!("{:?}", trans);

        assert!(trans.is_finite());
    }
}