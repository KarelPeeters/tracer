use std::cmp::min;
use std::time::Instant;

use imgref::ImgVec;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::common::progress::{Block, PixelResult, ProgressHandler};
use crate::common::scene::{Object, Scene};
use crate::cpu::accel::Accel;
use crate::cpu::renderer::{CpuPreparedScene, CpuRenderSettings};

pub struct CpuRenderer<P: ProgressHandler> {
    pub settings: CpuRenderSettings,
    pub progress_handler: P,
}

fn split_into_blocks(width: u32, height: u32) -> Vec<Block> {
    let block_size: u32 = 16;

    let mut result = Vec::new();
    for x in (0..width).step_by(block_size as usize) {
        for y in (0..height).step_by(block_size as usize) {
            result.push(Block {
                x,
                y,
                width: min(block_size, width - x),
                height: min(block_size, height - y),
            })
        }
    }

    result
}

impl<P: ProgressHandler> CpuRenderer<P> {
    pub fn render<A: Accel>(self, scene: &Scene, width: u32, height: u32, accel: impl FnOnce(&[Object]) -> A) -> ImgVec<PixelResult> {
        println!("Building accel");
        let start = Instant::now();
        let accel = accel(&scene.objects);
        println!("  {:?}", accel);
        println!("  took {:?}", start.elapsed());

        let prepared_scene = CpuPreparedScene::new(scene, self.settings, accel, width, height);

        let mut progress_handler = self.progress_handler.init(width, height);

        // channel to send results back to this thread
        let (sender, receiver) =
            crossbeam::channel::unbounded::<(Block, Vec<PixelResult>)>();

        // start the collector thread responsible to collecting the final output and reporting progress
        let builder = std::thread::Builder::new().name("collector".to_owned());
        let collector_handle = builder.spawn(move || {
            let target_buf = vec![PixelResult::default(); (width * height) as usize];
            let mut target = ImgVec::new(target_buf, width as usize, height as usize);

            for (block, pixels) in receiver.clone() {
                for dy in 0..block.height {
                    for dx in 0..block.width {
                        target[(block.x + dx, block.y + dy)] = pixels[(dy * block.width + dx) as usize];
                    }
                }

                P::update(&mut progress_handler, block, &pixels);
            }

            target
        }).expect("Failed to spawn collector thread");

        let mut blocks = split_into_blocks(width, height);
        blocks.shuffle(&mut thread_rng());

        // render everything on a thread pool, send data to the channel
        blocks.par_iter().panic_fuse().for_each_init(thread_rng, |rng, block: &Block| {
            let mut data = Vec::new();
            for y in block.y_range() {
                for x in block.x_range() {
                    data.push(prepared_scene.calculate_pixel(rng, x, y))
                }
            }

            sender.send((*block, data)).expect("Failed to send block result over channel");
        });

        drop(sender);

        let result = collector_handle.join()
            .expect("Joining collector thread deadlocked?");
        result
    }
}