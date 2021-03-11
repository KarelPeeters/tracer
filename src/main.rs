#![allow(dead_code)]


use std::{fs, io};
use std::cmp::max;
use std::path::PathBuf;
use std::time::Instant;

use imgref::Img;

use crate::common::Renderer;
use crate::common::scene::Color;
use crate::common::util::to_image;
use crate::cpu::{CpuRenderer, Strategy};

pub mod common;
pub mod cpu;

mod demos;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scene = demos::colored_spheres();

    let renderer = CpuRenderer {
        sample_count: 100_000,
        max_bounces: 8,
        anti_alias: true,
        strategy: Strategy::Simple,
    };

    let div = 8;
    let (width, height) = (1920 / div, 1080 / div);

    let mut result = Img::new(vec![Color::default(); width * height], width, height);

    let start = Instant::now();
    renderer.render(&scene, result.as_mut());
    println!("Render took {:?}s", (Instant::now() - start).as_secs_f32());

    let (result_clipped, _) = to_image(result.as_ref());
    let output_paths = [PathBuf::from("ignored/output"), pick_output_file_path()?];

    for output_path in output_paths.iter() {
        println!("Saving output to {:?}", output_path);

        fs::write(output_path.with_extension("txt"), &format!("{:#?}", renderer).as_bytes())?;

        exr::prelude::write_rgb_file(output_path.with_extension("exr"), width, height, |x, y| {
            let color = result[(x, y)];
            (color.red, color.green, color.blue)
        }).expect("Failed to save exf image");

        result_clipped.save(output_path.with_extension("png"))?;
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