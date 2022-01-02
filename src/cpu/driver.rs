use std::cmp::min;

use imgref::ImgVec;
use rand::thread_rng;
use rayon::iter::{ParallelIterator, IntoParallelIterator};

use crate::common::scene::Scene;
use crate::cpu::renderer::{CpuRenderSettings, PixelResult, RayCamera};
use rand::prelude::SliceRandom;

pub struct CpuRenderer<P: ProgressHandler> {
    pub settings: CpuRenderSettings,
    pub progress_handler: P,
}

pub trait ProgressHandler: Send {
    type State: Send + 'static;
    fn init(self, width: u32, height: u32) -> Self::State;
    fn update(state: &mut Self::State, block: Block, pixels: &Vec<PixelResult>);
}

#[derive(Debug, Copy, Clone)]
pub struct Block {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

fn split_into_blocks(width: u32, height: u32) -> Vec<Block> {
    //TODO do some cool space-filling curve or something here, maybe even just a spiral
    //  having an order to this requires changing the rayon code to actually process things in order though

    let block_size: u32 = 8;

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

    result.shuffle(&mut thread_rng());
    result
}

impl<P: ProgressHandler> CpuRenderer<P> {
    pub fn render(self, scene: &Scene, width: u32, height: u32) -> ImgVec<PixelResult> {
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

        let settings = self.settings;

        let camera = RayCamera::new(&scene.camera, settings.anti_alias, width, height);

        let blocks = split_into_blocks(width, height);
        let block_count = blocks.len();

        let (block_sender, block_receiver) =
            crossbeam::channel::unbounded::<Block>();
        for block in blocks {
            block_sender.send(block).expect("Failed to send block");
        }
        drop(block_sender);

        // render everything on a thread pool, send data to the channel
        (0..block_count).into_par_iter().panic_fuse().for_each_init(thread_rng, |rng, _| {
            let block = block_receiver.recv().expect("Failed to receive block");
            let mut data = Vec::new();
            for y in block.y..(block.y + block.height) {
                for x in block.x..(block.x + block.width) {
                    data.push(settings.calculate_pixel(scene, &camera, rng, x, y))
                }
            }

            sender.send((block, data)).expect("Failed to send block result over channel");
        });

        drop(sender);

        let result = collector_handle.join()
            .expect("Joining collector thread deadlocked?");
        result
    }
}

pub struct PrintProgress;

pub struct PrintProgressState {
    total_pixels: u64,
    finished_pixels: u64,
    prev_printed: f32,
}

impl ProgressHandler for PrintProgress {
    type State = PrintProgressState;

    fn init(self, width: u32, height: u32) -> Self::State {
        println!("Progress {:.03}", 0.0);

        PrintProgressState {
            total_pixels: (width as u64) * (height as u64),
            finished_pixels: 0,
            prev_printed: 0.0,
        }
    }

    fn update(state: &mut Self::State, block: Block, _: &Vec<PixelResult>) {
        state.finished_pixels += (block.width as u64) * (block.height as u64);
        let progress = (state.finished_pixels as f32) / (state.total_pixels as f32);
        if progress - state.prev_printed >= 0.01 || progress == 1.0 {
            state.prev_printed = progress;
            println!("Progress {:.03}", progress);
        }
    }
}

pub struct CombinedProgress<L: ProgressHandler, R: ProgressHandler> {
    left: L,
    right: R,
}

impl<L: ProgressHandler, R: ProgressHandler> CombinedProgress<L, R> {
    pub fn new(left: L, right: R) -> Self {
        CombinedProgress { left, right }
    }
}

impl<L: ProgressHandler, R: ProgressHandler> ProgressHandler for CombinedProgress<L, R> {
    type State = (L::State, R::State);

    fn init(self, width: u32, height: u32) -> Self::State {
        (L::init(self.left, width, height), R::init(self.right, width, height))
    }

    fn update(state: &mut Self::State, block: Block, pixels: &Vec<PixelResult>) {
        L::update(&mut state.0, block, pixels);
        R::update(&mut state.1, block, pixels);
    }
}