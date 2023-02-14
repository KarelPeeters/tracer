use exr::image::{Image, Layer, SpecificChannels};
use exr::image::write::channels::GetPixel;
use exr::math::Vec2;
use exr::meta::attribute::{ChannelDescription, SampleType};
use imgref::ImgRef;

use crate::common::scene::Color;
use crate::cpu::PixelResult;

type DiscreteImage = image::ImageBuffer<image::Rgb<u8>, Vec<u8>>;

/// Convert the given image to a format suitable for saving to a png file.
/// The first return Image is the image itself, the second Image shows where values had to be clipped
/// to fit into the image format .
pub fn to_discrete_image(image: ImgRef<PixelResult>) -> (DiscreteImage, DiscreteImage) {
    let mut result = DiscreteImage::new(image.width() as u32, image.height() as u32);
    let mut clipped = DiscreteImage::new(image.width() as u32, image.height() as u32);

    let max = palette::Srgb::new(1.0, 1.0, 1.0).into_linear();

    for (x, y, p) in result.enumerate_pixels_mut() {
        let linear: Color = image[(x, y)].color;

        let srgb = palette::Srgb::from_linear(linear);
        let data = srgb.into_format();

        *p = image::Rgb([data.red, data.green, data.blue]);
        clipped[(x, y)] = image::Rgb([
            if linear.red > max.red { 255 } else { 0 },
            if linear.green > max.green { 255 } else { 0 },
            if linear.blue > max.blue { 255 } else { 0 },
        ]);
    }

    (result, clipped)
}

pub struct ImageWrapper<'a>(ImgRef<'a, PixelResult>);
pub type ChannelTuple = (ChannelDescription, ChannelDescription, ChannelDescription, ChannelDescription, ChannelDescription, ChannelDescription, ChannelDescription, ChannelDescription, ChannelDescription, ChannelDescription);

/// Convert the given image to the exr file format.
pub fn to_exr_image(image: ImgRef<PixelResult>) -> Image<Layer<SpecificChannels<ImageWrapper, ChannelTuple>>> {
    impl GetPixel for ImageWrapper<'_> {
        type Pixel = (f32, f32, f32, f32, f32, f32, f32, f32, f32, f32);

        fn get_pixel(&self, Vec2(x, y): Vec2<usize>) -> Self::Pixel {
            let pixel = self.0[(x, y)];
            (
                pixel.color.red, pixel.color.green, pixel.color.blue,
                pixel.variance.red, pixel.variance.green, pixel.variance.blue,
                pixel.rel_variance.red, pixel.rel_variance.green, pixel.rel_variance.blue,
                pixel.samples as f32,
            )
        }
    }

    let channels = SpecificChannels {
        channels: (
            ChannelDescription::named("R", SampleType::F32),
            ChannelDescription::named("G", SampleType::F32),
            ChannelDescription::named("B", SampleType::F32),
            ChannelDescription::named("var0-R", SampleType::F32),
            ChannelDescription::named("var1-G", SampleType::F32),
            ChannelDescription::named("var2-B", SampleType::F32),
            ChannelDescription::named("rel0-R", SampleType::F32),
            ChannelDescription::named("rel1-G", SampleType::F32),
            ChannelDescription::named("rel2-B", SampleType::F32),
            ChannelDescription::named("samples", SampleType::F32),
        ),
        pixels: ImageWrapper(image),
    };

    exr::image::Image::from_channels((image.width(), image.height()), channels)
}