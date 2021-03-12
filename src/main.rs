#![allow(dead_code)]

use std::{fs, io};
use std::cmp::max;
use std::path::PathBuf;
use std::time::Instant;

use exr::prelude::WritableImage;
use imgref::ImgRef;

use crate::common::scene::Color;
use crate::cpu::{CpuRenderer, PixelResult, StopCondition, Strategy};

pub mod common;
pub mod cpu;

mod demos;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scene = demos::colored_spheres();

    let renderer = CpuRenderer {
        stop_condition: StopCondition::SampleCount(1000),
        max_bounces: 8,
        anti_alias: true,
        strategy: Strategy::SampleLights,
        print_progress: true,
    };

    let div = 8;
    let (width, height) = (1920 / div, 1080 / div);

    let start = Instant::now();
    let image = renderer.render(&scene, width, height);
    println!("Render took {:?}s", (Instant::now() - start).as_secs_f32());

    let info = format!("{:#?}\n\n{:#?}", renderer, scene);
    let (image_discrete, _) = to_discrete_image(image.as_ref());

    let output_paths = [PathBuf::from("ignored/output"), pick_output_file_path()?];
    for output_path in output_paths.iter() {
        println!("Saving output to {:?}", output_path);

        fs::write(output_path.with_extension("txt"), info.as_bytes())?;

        save_exr_image(image.as_ref(), output_path.with_extension("exr"))?;

        image_discrete
            .save(output_path.with_extension("png"))?;
    }

    Ok(())
}


fn pick_output_file_path() -> io::Result<PathBuf> {
    fs::create_dir_all("ignored/output")?;

    let max_int: io::Result<u32> = fs::read_dir("ignored/output")?.try_fold(0, |a, entry| {
        let entry = entry?;

        let x = entry.path().file_stem().unwrap_or_default()
            .to_string_lossy()
            .parse::<u32>().unwrap_or(0);
        Ok(max(a, x))
    });

    let next_int = max_int? + 1;
    let path = ["ignored", "output", &next_int.to_string()].iter().collect();
    Ok(path)
}

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

    return (result, clipped);
}

fn save_exr_image(image: ImgRef<PixelResult>, path: impl AsRef<std::path::Path>) -> exr::error::Result<()> {
    //TODO add channels for relative variance when we figure out how to achieve that in exr
    let channels = exr::image::SpecificChannels::build()
        .with_channel("R")
        .with_channel("G")
        .with_channel("B")
        .with_channel("var0-R")
        .with_channel("var1-G")
        .with_channel("var2-B")
        .with_channel("samples")
        .with_pixel_fn(|exr::math::Vec2(x, y)| {
            let p = image[(x, y)];
            (
                p.color.red, p.color.green, p.color.blue,
                p.variance.red, p.variance.green, p.variance.blue,
                p.samples
            )
        });

    let img = exr::image::Image::from_channels((image.width(), image.height()), channels);
    img.write().to_file(path)
}