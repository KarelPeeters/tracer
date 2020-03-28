#![allow(dead_code)]

use image::{ImageBuffer, Rgb, Rgba};
use material::Material;
use nalgebra::{Matrix3x2, sup, Unit};
use palette::{LinSrgb, Srgb};
use rand::{Rng, thread_rng};
use std::fs::read_to_string;
use std::sync::Arc;
use std::time::Instant;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, ImmutableBuffer};
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::PipelineLayoutAbstract;
use vulkano::device::{Device, DeviceExtensions, Features};
use vulkano::format::Format;
use vulkano::image::{Dimensions, StorageImage};
use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};
use vulkano::pipeline::ComputePipeline;
use vulkano::sync;
use vulkano::sync::GpuFuture;
use wavefront_obj::obj;
use wavefront_obj::obj::{Primitive, Vertex};

mod material;

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/main.glsl"
    }
}

type Vec3 = nalgebra::Vector3<f32>;
type Point3 = nalgebra::Point3<f32>;

const GLASS_REFRACT: f32 = 1.0 / 1.52;

fn scale_lsrgb(color: [f32; 3], f: f32) -> [f32; 3] {
    [color[0] * f, color[1] * f, color[2] * f]
}

fn srgb(r: u8, g: u8, b: u8) -> [f32; 3] {
    srgb_f32(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

fn srgb_f32(r: f32, g: f32, b: f32) -> [f32; 3] {
    let lin = Srgb::new(r, g, b).into_linear();
    [lin.red, lin.green, lin.blue]
}

fn pow(color: [f32; 3], e: f32) -> [f32; 3] {
    [color[0].powf(e), color[1].powf(e), color[2].powf(e)]
}

fn vertex_to_point(vertex: &Vertex) -> Point3 {
    Point3::new(vertex.x as f32, vertex.y as f32, vertex.z as f32)
}

fn obj_to_triangles(obj: &obj::Object) -> Vec<cs::ty::Triangle> {
    let mut result = Vec::new();

    for geometry in &obj.geometry {
        for shape in &geometry.shapes {
            match shape.primitive {
                Primitive::Point(_) => {}
                Primitive::Line(_, _) => {}
                Primitive::Triangle((avi, _, ani), (bvi, ..), (cvi, ..)) => {
                    let a = vertex_to_point(&obj.vertices[avi]);
                    let b = vertex_to_point(&obj.vertices[bvi]);
                    let c = vertex_to_point(&obj.vertices[cvi]);

                    let an = vertex_to_point(&obj.normals[ani.unwrap()]);

                    let db = &b - &a;
                    let dc = &c - &a;

                    let normal = Unit::new_normalize(db.cross(&dc));

                    let project = Matrix3x2::from_columns(&[db, dc]).pseudo_inverse(0.0).expect("Degenerate triangle");

                    result.push(cs::ty::Triangle {
                        project: [
                            [*project.index((0, 0)), *project.index((1, 0))],
                            [*project.index((0, 1)), *project.index((1, 1))],
                            [*project.index((0, 2)), *project.index((1, 2))]
                        ],
                        point: [a.x as f32, a.y as f32, a.z as f32],
                        plane: cs::ty::Plane {
                            dist: normal.dot(&a.coords),
                            normal: [normal.x, normal.y, normal.z],
                            materialIndex: 3,
                            _dummy0: Default::default(),
                        },
                        renderNormal: [an.x as f32, an.y as f32, an.z as f32],
                        _dummy0: Default::default(),
                        _dummy1: Default::default(),
                        _dummy2: Default::default(),
                    })
                }
            }
        }
    }

    result
}

fn main() {
    let spheres: Vec<cs::ty::Sphere> = vec![
        //light
        cs::ty::Sphere {
            center: [10.0, 20.0, 10.0],
            radius: 3.0,

            materialIndex: 1,
            _dummy0: Default::default(),
        },
        //actual spheres
        cs::ty::Sphere {
            center: [-3.0, 3.0, 5.0],
            radius: 1.0,

            materialIndex: 2,
            _dummy0: Default::default(),
        },
        cs::ty::Sphere {
            center: [0.0, 3.0, 5.0],
            radius: 1.0,
            materialIndex: 3,

            _dummy0: Default::default(),
        },
        cs::ty::Sphere {
            center: [3.0, 3.0, 5.0],
            radius: 1.0,

            materialIndex: 4,
            _dummy0: Default::default(),
        }
    ];

    let planes: Vec<cs::ty::Plane> = vec![
        cs::ty::Plane {
            dist: 0.0,
            normal: [0.0, 1.0, 0.0],
            materialIndex: 0,

            _dummy0: Default::default(),
        },
    ];

    let lights: Vec<cs::ty::Light> = vec![
        cs::ty::Light {
            position: [10.0, 20.0, 10.0],
            _dummy0: Default::default(),
            color: srgb_f32(1.0, 1.0, 1.0),
            _dummy1: Default::default(),
        },
    ];

    let materials: Vec<cs::ty::Material> = vec![
        //floor material
        Material::Opaque {
            color: [0.9, 0.9, 0.9],

            mirror: 0.2,
            diffuse: 1.0,
        },

        //light material
        Material::Fixed {
            color: scale_lsrgb(srgb_f32(1.0, 1.0, 1.0), 50.0),
        },

        //object materials
        Material::Opaque {
            color: srgb(255, 0, 0),

            mirror: 10.0,
            diffuse: 10.0,
        },
        Material::Transparent {
            surface_color: [1.0, 1.0, 1.0],
            volumetric_color: pow(srgb(150, 150, 255), 1.0),
            refract_ratio: GLASS_REFRACT,
            scatter_coef: 0.0,

            mirror: 0.0,
            diffuse: 0.0,
            transparent: 10.0,
        },
        Material::Opaque {
            color: srgb(0, 128, 0),

            mirror: 10.0,
            diffuse: 10.0,
        },
    ].iter().map(|m| m.as_cs_ty()).collect();

    let mut triangles: Vec<cs::ty::Triangle> = vec![
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

    let obj_str = read_to_string("ignored/models/cube.obj").expect("Error while reading model");
    let obj_set = obj::parse(obj_str).expect("Error while parsing model");
    let obj_triangles = obj_to_triangles(obj_set.objects.first().expect("No object found"));
    // triangles.extend(obj_triangles);

    let super_sample_count: usize = 1;

    let width = 1024;
    let height = 512;
    let aspect_ratio = (width as f32) / (height as f32);

    let specialization_constants = cs::SpecializationConstants {
        MAX_BOUNCES: 8,
    };

    let push_constants = cs::ty::PushConstants {
        CAMERA: cs::ty::Camera {
            position: [0.0, 1.5, -12.0],
            direction: [0.0, 0.0, 1.0],
            focusDistance: 8.0 + 5.0 - 5.0,
            aperture: 0.0,
            aspectRatio: aspect_ratio,
            startScatteringCoef: 0.1,
            startVolumetricMask: [1.0, 1.0, 1.0],

            _dummy0: Default::default(),
            _dummy1: Default::default(),
        },
        SKY_COLOR: [0.1, 0.1, 0.1], //[0.529 / 2.0, 0.808 / 2.0, 0.922 / 2.0],
        SAMPLE_COUNT: 2000,
        SAMPLE_LIGHTS: false as u32,
    };

    let instance = Instance::new(None, &InstanceExtensions::none(), None)
        .expect("failed to create instance");

    for physical in PhysicalDevice::enumerate(&instance) {
        println!("Name: {:?}", physical.name());
    }

    let physical = PhysicalDevice::enumerate(&instance).next()
        .expect("no device found");
    println!("Picked {:?}", physical.name());
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

    let image = StorageImage::new(device.clone(), Dimensions::Dim2d { width, height }, Format::R8G8B8A8Uint, Some(queue_family))
        .expect("failed to create image");

    let shader = cs::Shader::load(device.clone())
        .expect("failed to create shader");

    let compute_pipeline = Arc::new(ComputePipeline::new(device.clone(), &shader.main_entry_point(), &specialization_constants)
        .expect("failed to create pipeline"));

    let usage = BufferUsage { storage_buffer: true, ..BufferUsage::none() };

    let (materials_buffer, materials_fut) = ImmutableBuffer::from_iter(materials.iter().cloned(), usage, queue.clone()).unwrap();
    let (lights_buffer, lights_fut) = ImmutableBuffer::from_iter(lights.iter().cloned(), usage, queue.clone()).unwrap();

    let (spheres_buffer, spheres_fut) = ImmutableBuffer::from_iter(spheres.iter().cloned(), usage, queue.clone()).unwrap();
    let (planes_buffer, planes_fut) = ImmutableBuffer::from_iter(planes.iter().cloned(), usage, queue.clone()).unwrap();
    let (triangles_buffer, triangles_fut) = ImmutableBuffer::from_iter(triangles.iter().cloned(), usage, queue.clone()).unwrap();

    materials_fut.join(lights_fut).join(spheres_fut).join(planes_fut).join(triangles_fut)
        .then_signal_fence_and_flush().unwrap()
        .wait(None).unwrap();

    let layout = compute_pipeline.layout().descriptor_set_layout(0).unwrap();

    let set = Arc::new(PersistentDescriptorSet::start(layout.clone())
        .add_image(image.clone()).unwrap()
        .add_buffer(materials_buffer.clone()).unwrap()
        .add_buffer(lights_buffer.clone()).unwrap()
        .add_buffer(spheres_buffer.clone()).unwrap()
        .add_buffer(planes_buffer.clone()).unwrap()
        .add_buffer(triangles_buffer.clone()).unwrap()
        .build().unwrap());

    let mut result = vec![vec![LinSrgb::new(0.0, 0.0, 0.0); height as usize]; width as usize];
    let mut rng = thread_rng();

    for i in 0..super_sample_count {
        // let mut write = result_buffer.write().unwrap();
        // for j in 0..result_size {
        //     write[j as usize] = rng.gen();
        // }
        // drop(write);

        println!("Super sample {}/{}", i, super_sample_count);

        let result_size = width * height * 4;
        let result_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), true, (0..result_size).map(|_| rng.gen()))
            .expect("failed to create buffer");

        let gpu_start = Instant::now();
        let command_buffer = AutoCommandBufferBuilder::primary_simultaneous_use(device.clone(), queue_family).unwrap()
            .copy_buffer_to_image(result_buffer.clone(), image.clone()).unwrap()
            .dispatch([width / 8, height / 8, 1], compute_pipeline.clone(), set.clone(), push_constants).unwrap()
            .copy_image_to_buffer(image.clone(), result_buffer.clone()).unwrap()
            .build().unwrap();

        sync::now(device.clone())
            .then_execute(queue.clone(), command_buffer).unwrap()
            .then_signal_fence_and_flush().unwrap()
            .wait(None).unwrap();

        println!("GPU Calculation took {}s", (Instant::now() - gpu_start).as_secs_f32());

        let buffer_content = result_buffer.read().unwrap();
        let image = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, &*buffer_content).unwrap();

        for (x, y, pix) in image.enumerate_pixels() {
            result[x as usize][y as usize] +=
                LinSrgb::new(pix.0[0], pix.0[1], pix.0[2]).into_format()
                    / (super_sample_count as f32);
        }
    }

    let image = ImageBuffer::<Rgb<u8>, _>::from_fn(width, height, |x, y| {
        let color = Srgb::from_linear(result[x as usize][y as usize]).into_format();
        image::Rgb([color.red, color.green, color.blue])
    });
    image.save("ignored/output.png").unwrap();
}