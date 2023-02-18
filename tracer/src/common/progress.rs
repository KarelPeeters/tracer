use std::ops::Range;
use std::time::{Duration, Instant};
use crate::common::scene::Color;

#[derive(Debug, Copy, Clone)]
pub struct Block {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct PixelResult {
    pub color: Color,
    pub variance: Color,
    pub rel_variance: Color,
    pub samples: u32,
}

//TODO write a proper iterator for the coords in Block instead
//  * iterate over y slower than x!
//  * be careful about empty x/y ranges!
//  * write a custom fold and size_hint ugh this is getting annoying?
impl Block {
    pub fn x_range(self) -> Range<u32> {
        self.x..(self.x + self.width)
    }

    pub fn y_range(self) -> Range<u32> {
        self.y..(self.y + self.height)
    }
}

pub trait ProgressHandler: Send {
    type State: Send + 'static;
    fn init(self, width: u32, height: u32) -> Self::State;
    fn update(state: &mut Self::State, block: Block, pixels: &Vec<PixelResult>);
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
