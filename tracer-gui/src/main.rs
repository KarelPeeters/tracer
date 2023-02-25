use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use eframe::{CreationContext, egui, Frame};
use eframe::egui::{Color32, ColorImage, Context, SidePanel, Slider, TextureHandle, TextureOptions, Vec2};
use once_cell::sync::OnceCell;
use rand::{Rng, SeedableRng};
use rand::rngs::SmallRng;

use tracer::common::scene::{Color, Scene};
use tracer::cpu::{CpuPreparedScene, CpuRenderSettings, StopCondition, Strategy};
use tracer::cpu::accel::NoAccel;
use tracer::cpu::stats::ColorVarianceEstimator;
use tracer::demos;

const SYNC_UPDATE_FREQ: usize = 64;

fn main() -> eframe::Result<()> {
    let scene = demos::colored_spheres();

    let image = Arc::new(Mutex::new(SharedImage::new(1920, 1080)));
    let stop = AtomicBool::new(false);

    std::thread::scope(|s| {
        let image_clone = image.clone();
        let stop_ref = &stop;
        let scene_ref = &scene;
        s.spawn(move || {
            renderer_main(scene_ref, image_clone, stop_ref);
        });

        eframe::run_native(
            "app name",
            eframe::NativeOptions::default(),
            Box::new(move |cc| Box::new(App::new(cc, image.clone()))),
        ).unwrap();

        stop.store(true, Ordering::Relaxed);
    });

    Ok(())
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct ImageSettings {
    exposure: f32,
    texture: TextureOptions,
}

impl Default for ImageSettings {
    fn default() -> Self {
        ImageSettings {
            exposure: 0.0,
            texture: TextureOptions::NEAREST,
        }
    }
}

impl ImageSettings {
    fn map(&self, color: Color) -> Color {
        color * 2f32.powf(self.exposure)
    }
}

struct SharedImage {
    width: u32,
    height: u32,

    buffer: Vec<ColorVarianceEstimator>,
    buffer_changed: bool,

    prev_settings: Option<ImageSettings>,
    prev_texture: Option<TextureHandle>,
    ctx: OnceCell<Context>,
}

impl SharedImage {
    fn new(width: u32, height: u32) -> Self {
        SharedImage {
            width,
            height,
            buffer: vec![Default::default(); (width * height) as usize],
            buffer_changed: false,
            prev_settings: None,
            prev_texture: None,
            ctx: OnceCell::new(),
        }
    }

    fn set_pixel(&mut self, x: u32, y: u32, value: ColorVarianceEstimator) {
        self.buffer[y as usize * self.width as usize + x as usize] = value;
    }

    fn get_pixel(&self, x: u32, y: u32) -> &ColorVarianceEstimator {
        &self.buffer[y as usize * self.width as usize + x as usize]
    }

    fn mark_changed(&mut self) {
        self.buffer_changed = true;
        if let Some(ctx) = self.ctx.get() {
            ctx.request_repaint();
        }
    }

    fn as_texture(&mut self, ctx: &Context, name: &str, settings: ImageSettings) -> TextureHandle {
        let _ = self.ctx.set(ctx.clone());

        let changed = self.buffer_changed | (self.prev_settings != Some(settings));
        self.prev_settings = Some(settings);
        self.buffer_changed = false;

        match (&self.prev_texture, changed) {
            (Some(texture), false) => texture.clone(),
            (None, _) | (Some(_), true) => {
                let image = self.to_image(settings);
                let texture = ctx.load_texture(name, image, settings.texture);
                self.prev_texture = Some(texture.clone());
                texture
            }
        }
    }

    fn to_image(&self, settings: ImageSettings) -> ColorImage {
        let start = Instant::now();
        let mut image = ColorImage::new([self.width as usize, self.height as usize], Color32::BLACK);

        for y in 0..self.height {
            for x in 0..self.width {
                let color_orig = self.get_pixel(x, y).mean;
                let color_mapped = settings.map(color_orig);

                let color_srgb = palette::Srgb::from_linear(color_mapped);
                let color_byte = color_srgb.into_format();

                let color_32 = Color32::from_rgb(color_byte.red, color_byte.green, color_byte.blue);
                image[(x as usize, y as usize)] = color_32;
            }
        }

        println!("to_image took {}s", start.elapsed().as_secs_f32());
        image
    }
}

fn renderer_main(scene: &Scene, image: Arc<Mutex<SharedImage>>, stop: &AtomicBool) {
    let (width, height) = {
        let image = image.lock().unwrap();
        (image.width, image.height)
    };

    let settings = CpuRenderSettings {
        stop_condition: StopCondition::SampleCount(0),
        max_bounces: 8,
        anti_alias: true,
        strategy: Strategy::SampleLights,
    };

    // let accel = BVH::new(&scene.objects, Default::default());
    let accel = NoAccel;

    let prepared = CpuPreparedScene::new(&scene, settings, accel, width, height);

    let mut buffer = vec![ColorVarianceEstimator::default(); (width * height) as usize];

    let mut updates = vec![];

    let mut samples = 0;
    let mut prev = Instant::now();

    let mut rng = SmallRng::from_entropy();

    loop {
        let x = rng.gen_range(0..width);
        let y = rng.gen_range(0..height);
        let color = prepared.sample_pixel(&mut rng, x, y);

        let estimator = &mut buffer[(y * width + x) as usize];
        estimator.update(color);
        updates.push((x, y, estimator.clone()));

        samples += 1;

        if updates.len() >= SYNC_UPDATE_FREQ {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            if prev.elapsed().as_secs() >= 1 {
                println!("throughput: {} rays/s", samples as f32 / prev.elapsed().as_secs_f32());
                prev = Instant::now();
                samples = 0;
            }

            let mut image = image.lock().unwrap();
            for (x, y, c) in updates.drain(..) {
                image.set_pixel(x, y, c);
            }
            image.mark_changed();
        }
    }
}

struct App {
    image: Arc<Mutex<SharedImage>>,
    ctx: OnceCell<Context>,

    settings: ImageSettings,
}

impl App {
    pub fn new(_: &CreationContext, image: Arc<Mutex<SharedImage>>) -> Self {
        App {
            image,
            ctx: OnceCell::new(),
            settings: ImageSettings::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _: &mut Frame) {
        // TODO avoid clone here?
        let _ = self.ctx.set(ctx.clone());

        let (texture, width, height) = {
            let start = Instant::now();
            let mut image = self.image.lock().unwrap();
            let lock_time = start.elapsed();

            let start = Instant::now();
            let texture = image.as_texture(ctx, "image", self.settings);
            let texture_time = start.elapsed();

            println!("lock too {}s, texture {}s", lock_time.as_secs_f32(), texture_time.as_secs_f32());

            (texture, image.width, image.height)
        };

        SidePanel::left("side_panel").show(ctx, |ui| {
            ui.add(Slider::new(&mut self.settings.exposure, -5.0..=5.0));
        });

        // TODO stop this from overriding the side panel
        egui::Area::new("area").show(ctx, |ui| {
            ui.image(texture.id(), Vec2::new(width as f32, height as f32))
        });
    }
}
