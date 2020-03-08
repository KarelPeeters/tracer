#![allow(dead_code)]


use std::sync::Arc;
use std::time::Instant;

use image::{ImageBuffer, Rgba};
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::PipelineLayoutAbstract;
use vulkano::device::{Device, DeviceExtensions, Features};
use vulkano::format::Format;
use vulkano::image::{Dimensions, StorageImage};
use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};
use vulkano::pipeline::ComputePipeline;
use vulkano::sync::GpuFuture;
use rand::{thread_rng, Rng};

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/main.glsl"
    }
}

fn first_factor(x: u32) -> u32 {
    let mut result = 0;
    let mut i = 2;

    while i * i < 3162 {
        if x % i == 0 { result = i }
        i += 1;
    }

    result
}

fn main() {
    let spheres = vec![
        cs::ty::Sphere {
            center: [-3.0, 1.0, 5.0],
            radius: 1.0,

            materialIndex: 0,
            _dummy0: Default::default(),
        },
        cs::ty::Sphere {
            center: [0.0, 1.0, 5.0],
            radius: 1.0,
            materialIndex: 0,

            _dummy0: Default::default(),
        },
        cs::ty::Sphere {
            center: [3.0, 1.0, 5.0],
            radius: 1.0,

            materialIndex: 0,
            _dummy0: Default::default(),
        }
    ];

    let planes: Vec<cs::ty::Plane> = vec![
        cs::ty::Plane {
            dist: 0.0,
            normal: [0.0, 1.0, 0.0],
            materialIndex: 1,

            _dummy0: Default::default(),
        },
    ];

    let lights = vec![
        cs::ty::Light {
            position: [10.0, 20.0, 20.0],
            _dummy0: Default::default(),
            color: [1.0, 1.0, 0.9],
            _dummy1: Default::default(),
        },
    ];

    let materials = vec![
        cs::ty::Material {
            color: [0.8, 0.8, 0.8],
            mirror: 0.9
        },
        cs::ty::Material {
            color: [0.5, 0.5, 0.5],
            mirror: 0.0
        },
        cs::ty::Material {
            color: [0.1, 0.1, 0.8],
            mirror: 0.0
        }
    ];

    let triangles: Vec<cs::ty::Triangle> = vec![
        /*cs::ty::Triangle {
            plane: cs::ty::Plane {
                dist: 3.0,
                normal: [0.0, 0.0, 1.0],
                materialIndex: 2,

                _dummy0: Default::default(),
            },
            point: [0.0, 0.0, 0.0],
            project: [[1.0, 0.0], [0.0, 1.0], [0.0, 0.0]],
            _dummy0: Default::default(),
            _dummy1: Default::default(),
        }*/
    ];

    let specialization_constants = cs::SpecializationConstants {
        MAX_BOUNCES: 8,
    };
    let push_constants = cs::ty::PushConstants {
        CAMERA_POS: [0.0, 2.5, -4.5],
        SKY_COLOR: [135.0/255.0, 206.0/255.0, 235.0/255.0],
        SAMPLE_COUNT: 100,
        _dummy0: Default::default(),
    };

    let instance = Instance::new(None, &InstanceExtensions::none(), None)
        .expect("failed to create instance");

    let physical = PhysicalDevice::enumerate(&instance).next()
        .expect("no device found");
    println!("Name: {:?}", physical.name());
    println!("Type: {:?}", physical.ty());
    println!("Max group count: {:?}", physical.limits().max_compute_work_group_count());
    println!("Max group size: {:?}", physical.limits().max_compute_work_group_size());

    let queue_family = physical.queue_families()
        .find(|&q| q.supports_compute())
        .expect("couldn't find compute family");

    let (device, mut queues) =
        Device::new(physical, &Features { shader_f3264: true, ..Features::none() },
                    &DeviceExtensions { khr_storage_buffer_storage_class: true, ..DeviceExtensions::none() },
                    [(queue_family, 0.5)].iter().cloned()).expect("failed to create device");

    let queue = queues.next().unwrap();

    let size = 1024;

    let image = StorageImage::new(device.clone(), Dimensions::Dim2d { width: size, height: size }, Format::R8G8B8A8Uint, Some(queue_family))
        .expect("failed to create image");

    let shader = cs::Shader::load(device.clone())
        .expect("failed to create shader");

    let compute_pipeline = Arc::new(ComputePipeline::new(device.clone(), &shader.main_entry_point(), &specialization_constants)
        .expect("failed to create pipeline"));

    let material_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, materials.iter().cloned()).unwrap();
    let lights_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, lights.iter().cloned()).unwrap();

    let spheres_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, spheres.iter().cloned()).unwrap();
    let planes_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, planes.iter().cloned()).unwrap();
    let triangles_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, triangles.iter().cloned()).unwrap();

    let layout = compute_pipeline.layout().descriptor_set_layout(0).unwrap();

    let set = Arc::new(PersistentDescriptorSet::start(layout.clone())
        .add_image(image.clone()).unwrap()
        .add_buffer(material_buffer.clone()).unwrap()
        .add_buffer(lights_buffer.clone()).unwrap()
        .add_buffer(spheres_buffer.clone()).unwrap()
        .add_buffer(planes_buffer.clone()).unwrap()
        .add_buffer(triangles_buffer.clone()).unwrap()
        .build().unwrap());

    let mut rng = thread_rng();
    let result_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), true, (0..size * size * 4).map(|_| rng.gen()))
        .expect("failed to create buffer");

    let gpu_start = Instant::now();
    let command_buffer = AutoCommandBufferBuilder::new(device.clone(), queue_family).unwrap()
        .copy_buffer_to_image(result_buffer.clone(), image.clone()).unwrap()
        .dispatch([size / 8, size / 8, 1], compute_pipeline.clone(), set.clone(), push_constants).unwrap()
        .copy_image_to_buffer(image.clone(), result_buffer.clone()).unwrap()
        .build().unwrap();

    let finished = command_buffer.execute(queue.clone()).unwrap();
    finished.then_signal_fence_and_flush().unwrap().wait(None).unwrap();
    println!("GPU Calculation took {}s", (Instant::now() - gpu_start).as_secs_f32());

    let buffer_content = result_buffer.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(size, size, &*buffer_content).unwrap();
    image.save("ignored/output.png").unwrap();
}