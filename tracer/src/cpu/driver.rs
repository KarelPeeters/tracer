use std::cmp::min;
use std::ops::Range;
use std::time::{Duration, Instant};

use imgref::ImgVec;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::common::scene::Scene;
use crate::cpu::accel::ObjectId;
use crate::cpu::accel::bvh::{BVH, BVHSplitStrategy};
use crate::cpu::renderer::{CpuRenderSettings, is_light, PixelResult, RayCamera, RenderStructure};

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

//TODO write a proper iterator for the coords in Block instead
//  * iterate over y slower than x!
//  * be careful about empty x/y ranges!
//  * write a custom fold and size_hint ugh this is getting annoying?
impl Block {
    fn x_range(self) -> Range<u32> {
        self.x..(self.x + self.width)
    }

    fn y_range(self) -> Range<u32> {
        self.y..(self.y + self.height)
    }
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
    pub fn render(self, scene: &Scene, width: u32, height: u32) -> ImgVec<PixelResult> {
        println!("Building accelerator");
        let start = Instant::now();
        let accel = BVH::new(&scene.objects, BVHSplitStrategy::default());
        // let accel = Octree::new(&scene.objects, self.settings.octree_max_flat_size);
        // let accel = NoAccel;
        println!("  {:?}", accel);
        println!("  took {:?}", start.elapsed());

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

        // pre-filter lights
        let lights = scene.objects.iter().enumerate().filter_map(|(id, object)| {
            if is_light(object) { Some(ObjectId::new(id)) } else { None }
        }).collect();

        let structure = RenderStructure {
            scene,
            camera,
            accel,
            lights,
            settings,
        };

        let mut blocks = split_into_blocks(width, height);
        blocks.shuffle(&mut thread_rng());

        // render everything on a thread pool, send data to the channel
        blocks.par_iter().panic_fuse().for_each_init(thread_rng, |rng, block: &Block| {
            let mut data = Vec::new();
            for y in block.y_range() {
                for x in block.x_range() {
                    data.push(structure.calculate_pixel(rng, x, y))
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

pub struct NoProgress;

impl ProgressHandler for NoProgress {
    type State = ();
    fn init(self, _: u32, _: u32) {}
    fn update(_: &mut Self::State, _: Block, _: &Vec<PixelResult>) {}
}

pub struct PrintProgress;

pub struct PrintProgressState {
    total_pixels: u64,
    finished_pixels: u64,
    prev_printed: f32,
    prev_time: Instant,
}

impl ProgressHandler for PrintProgress {
    type State = PrintProgressState;

    fn init(self, width: u32, height: u32) -> Self::State {
        println!("Progress {:.03}", 0.0);

        PrintProgressState {
            total_pixels: (width as u64) * (height as u64),
            finished_pixels: 0,
            prev_printed: f32::NEG_INFINITY,
            prev_time: Instant::now(),
        }
    }

    fn update(state: &mut Self::State, block: Block, _: &Vec<PixelResult>) {
        state.finished_pixels += (block.width as u64) * (block.height as u64);
        let progress = (state.finished_pixels as f32) / (state.total_pixels as f32);
        let delta = progress - state.prev_printed;

        if delta >= 0.01 || progress == 1.0 {
            let now = Instant::now();
            let elapsed = now - state.prev_time;
            let eta = Duration::try_from_secs_f32(elapsed.as_secs_f32() * (1.0 - progress) / delta).ok();

            println!("Progress {:.03}, eta {:.01?}", progress, eta);

            state.prev_printed = progress;
            state.prev_time = now;
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