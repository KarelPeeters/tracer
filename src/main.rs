#![allow(dead_code)]

use std::fs::read_to_string;
use std::sync::Arc;
use std::time::Instant;

use image::{ImageBuffer, Rgba};
use nalgebra::{Matrix3x2, Unit};
use rand::{Rng, thread_rng};
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
use wavefront_obj::obj;
use wavefront_obj::obj::{Primitive, Vertex};

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "shaders/main.glsl"
    }
}

type Vec3 = nalgebra::Vector3<f32>;
type Point3 = nalgebra::Point3<f32>;


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
                Primitive::Triangle((avi, ..), (bvi, ..), (cvi, ..)) => {
                    let a = vertex_to_point(&obj.vertices[avi]);
                    let b = vertex_to_point(&obj.vertices[bvi]);
                    let c = vertex_to_point(&obj.vertices[cvi]);

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
                            materialIndex: 0,
                            _dummy0: Default::default(),
                        },
                        _dummy0: Default::default(),
                        _dummy1: Default::default(),
                    })
                }
            }
        }
    }

    result
}

struct Material {
    color: [f32; 3],
    refract_ratio: f32,

    mirror: f32,
    diffuse: f32,
    transparent: f32,
}

impl Material {
    fn as_cs_ty(&self) -> cs::ty::Material {
        let total = self.mirror + self.diffuse + self.transparent;

        let r = cs::ty::Material {
            color: self.color,
            refractRatio: self.refract_ratio,
            keyDiffuse: self.diffuse / total,
            keyTransparent: 1.0 - self.transparent / total,

            _dummy0: Default::default(),
        };

        println!("{}", r.keyDiffuse);
        println!("{}", r.keyTransparent);

        r
    }
}

fn main() {
    let spheres: Vec<cs::ty::Sphere> = vec![
        cs::ty::Sphere {
            center: [-3.0, 1.0, 6.0],
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
            center: [3.0, 1.0, 4.0],
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

    let lights: Vec<cs::ty::Light> = vec![
        cs::ty::Light {
            position: [10.0, 20.0, -20.0],
            _dummy0: Default::default(),
            color: [0.5, 0.5, 0.5],
            _dummy1: Default::default(),
        },
    ];

    let materials = vec![
        //object material
        Material {
            color: [0.95, 0.95, 0.95],
            refract_ratio: 0.9,

            mirror: 1.0,
            diffuse: 1.0,
            transparent: 6.0,
        }.as_cs_ty(),
        //floor material
        Material {
            color: [0.9, 0.9, 0.9],
            refract_ratio: 1.0,

            mirror: 0.0,
            diffuse: 1.0,
            transparent: 0.0,
        }.as_cs_ty(),
    ];

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

    /*let obj_str = read_to_string("ignored/models/cube.obj").expect("Error while reading model");
    let obj_set = obj::parse(obj_str).expect("Error while parsing model");
    let obj_triangles = obj_to_triangles(obj_set.objects.first().expect("No object found"));
    triangles.extend(obj_triangles);*/

    let width = 1024;
    let height = 512;

    let aspect_ratio = (width as f32) / (height as f32);

    let specialization_constants = cs::SpecializationConstants {
        MAX_BOUNCES: 8,
    };

    let push_constants = cs::ty::PushConstants {
        CAMERA: cs::ty::Camera {
            position: [0.0, 1.5, -8.0],
            direction: [0.0, 0.0, 1.0],
            focusDistance: 8.0 + 5.0 - 5.0,
            aperture: 0.0,
            aspectRatio: aspect_ratio,

            _dummy0: Default::default(),
        },
        SKY_COLOR: [0.529 / 2.0, 0.808 / 2.0, 0.922 / 2.0],
        SAMPLE_COUNT: 1000,
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

    let image = StorageImage::new(device.clone(), Dimensions::Dim2d { width, height }, Format::R8G8B8A8Uint, Some(queue_family))
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
    let result_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), true, (0..width * height * 4).map(|_| rng.gen()))
        .expect("failed to create buffer");

    let gpu_start = Instant::now();
    let command_buffer = AutoCommandBufferBuilder::new(device.clone(), queue_family).unwrap()
        .copy_buffer_to_image(result_buffer.clone(), image.clone()).unwrap()
        .dispatch([width / 8, height / 8, 1], compute_pipeline.clone(), set.clone(), push_constants).unwrap()
        .copy_image_to_buffer(image.clone(), result_buffer.clone()).unwrap()
        .build().unwrap();

    let finished = command_buffer.execute(queue.clone()).unwrap();
    finished.then_signal_fence_and_flush().unwrap().wait(None).unwrap();
    println!("GPU Calculation took {}s", (Instant::now() - gpu_start).as_secs_f32());

    let buffer_content = result_buffer.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, &*buffer_content).unwrap();
    image.save("ignored/output.png").unwrap();
}