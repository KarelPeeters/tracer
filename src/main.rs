#![allow(dead_code)]


use std::time::Instant;

use imgref::Img;

use crate::common::Renderer;
use crate::common::scene::Color;
use crate::common::util::to_image;
use crate::cpu::CpuRenderer;

pub mod common;
pub mod cpu;

mod demos;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scene = demos::colored_spheres();

    let renderer = CpuRenderer {
        sample_count: 1000,
        max_bounces: 8,
        anti_alias: true,
    };

    let div = 8;
    let (width, height) = (1920 / div, 1080 / div);

    let mut result = Img::new(vec![Color::default(); width * height], width, height);

    let start = Instant::now();
    renderer.render(&scene, result.as_mut());
    println!("Render took {:?}s", (Instant::now() - start).as_secs_f32());

    exr::prelude::write_rgb_file("ignored/output.exr", width, height, |x, y| {
        let color = result[(x, y)];
        (color.red, color.green, color.blue)
    }).expect("Failed to save exf image");

    let (result, clipped) = to_image(result.as_ref());
    result.save("ignored/output.png")?;
    clipped.save("ignored/output_clipped.png")?;

    Ok(())
}
