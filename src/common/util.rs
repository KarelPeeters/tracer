use image::ImageBuffer;
use imgref::ImgRef;

use crate::common::scene::Color;

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
