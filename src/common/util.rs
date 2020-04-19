use image::ImageBuffer;
use imgref::ImgRef;
use nalgebra::{Matrix4};
use wavefront_obj::obj;
use wavefront_obj::obj::Primitive;

use crate::common::scene::{Color, Material, Object, Point3, Shape, Transform};
use alga::general::SubsetOf;

pub fn to_image(image: ImgRef<Color>) -> ImageBuffer<image::Rgb<u8>, Vec<u8>> {
    let mut result: ImageBuffer::<image::Rgb<u8>, Vec<u8>> = ImageBuffer::new(image.width() as u32, image.height() as u32);

    for (x, y, p) in result.enumerate_pixels_mut() {
        let linear: Color = image[(x, y)];
        let srgb = palette::Srgb::from_linear(linear);
        let data = srgb.into_format();
        *p = image::Rgb([data.red, data.green, data.blue]);
    }

    return result;
}

fn vertex_to_point(vertex: &obj::Vertex) -> Point3 {
    Point3::new(vertex.x as f32, vertex.y as f32, vertex.z as f32)
}

pub fn obj_to_triangles<W: SubsetOf<Transform>>(obj: &obj::Object, material: Material, transform: W) -> impl Iterator<Item=Object> + '_ {
    let transform = transform.to_superset();

    obj.geometry.iter().flat_map(move |geometry|
        geometry.shapes.iter().filter_map(move |shape| {
            match shape.primitive {
                Primitive::Point(_) => None,
                Primitive::Line(_, _) => None,
                Primitive::Triangle((avi, _, _), (bvi, ..), (cvi, ..)) => {
                    let a = vertex_to_point(&obj.vertices[avi]);
                    let b = vertex_to_point(&obj.vertices[bvi]);
                    let c = vertex_to_point(&obj.vertices[cvi]);

                    let db = &b - &a;
                    let dc = &c - &a;
                    let n = db.cross(&dc);

                    let local_transform = Transform::from_matrix_unchecked(Matrix4::new(
                        db.x, dc.x, n.x, a.x,
                        db.y, dc.y, n.y, a.y,
                        db.z, dc.z, n.z, a.z,
                        0.0, 0.0, 0.0, 1.0,
                    ));

                    Some(Object {
                        shape: Shape::Triangle,
                        material,
                        transform: (transform * local_transform).into(),
                    })
                }
            }
        })
    )
}
