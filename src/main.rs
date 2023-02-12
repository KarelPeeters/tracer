#![allow(dead_code)]

use std::{fs, io};
use std::cmp::max;
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Instant;

use exr::prelude::WritableImage;
use tev_client::TevClient;

use crate::common::util::lower_process_priority;
use crate::cpu::{CombinedProgress, CpuRenderer, CpuRenderSettings, PrintProgress, StopCondition, Strategy};
use crate::images::{to_discrete_image, to_exr_image};
use crate::tev::TevProgress;

pub mod common;
pub mod cpu;

mod demos;
mod tev;
mod images;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    lower_process_priority();
    // rayon::ThreadPoolBuilder::new().num_threads(1).build_global().unwrap();

    let scene = demos::random_tiles();

    let client = TevClient::wrap(TcpStream::connect("127.0.0.1:14158")?);

    let renderer = CpuRenderer {
        settings: CpuRenderSettings {
            stop_condition: StopCondition::SampleCount(10),
            max_bounces: 8,
            anti_alias: true,
            strategy: Strategy::SampleLights,
            octree_max_flat_size: 8,
        },
        progress_handler: CombinedProgress::new(
            PrintProgress,
            TevProgress::new("test", client),
        ),
    };

    let div = 1;
    let (width, height) = (1920 / div, 1080 / div);

    let settings = renderer.settings.clone();
    let start = Instant::now();
    let image = renderer.render(&scene, width, height);
    let elapsed = Instant::now() - start;
    println!("Render took {}s", elapsed.as_secs_f32());

    let info = format!("{:#?}\n\n{:#?}\n\nRender took {}s\n", settings, scene, elapsed.as_secs_f32());

    let (image_discrete, _) = to_discrete_image(image.as_ref());
    let image_exr = to_exr_image(image.as_ref());

    let output_paths = [PathBuf::from("ignored/output"), pick_output_file_path()?];
    for output_path in output_paths.iter() {
        println!("Saving output to {:?}", output_path);

        fs::write(output_path.with_extension("txt"), info.as_bytes())?;
        image_exr.write().to_file(output_path.with_extension("exr"))?;
        image_discrete.save(output_path.with_extension("png"))?;
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