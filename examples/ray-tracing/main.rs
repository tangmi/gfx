#[cfg(feature = "dx11")]
extern crate gfx_backend_dx11 as back;
#[cfg(feature = "dx12")]
extern crate gfx_backend_dx12 as back;
#[cfg(not(any(
    feature = "vulkan",
    feature = "dx11",
    feature = "dx12",
    feature = "metal",
    feature = "gl",
)))]
extern crate gfx_backend_empty as back;
#[cfg(feature = "gl")]
extern crate gfx_backend_gl as back;
#[cfg(feature = "metal")]
extern crate gfx_backend_metal as back;
#[cfg(feature = "vulkan")]
extern crate gfx_backend_vulkan as back;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn wasm_main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    main();
}

use hal::{
    acceleration_structure as accel, adapter, buffer, command, format, memory, pool, prelude::*,
    pso, window, IndexType,
};

use std::{iter, mem, ops, ptr};

#[cfg_attr(rustfmt, rustfmt_skip)]
const DIMS: window::Extent2D = window::Extent2D { width: 1024, height: 768 };

#[derive(Debug, Clone, Copy)]
#[allow(non_snake_case)]
struct Vertex {
    a_Pos: [f32; 3],
}

// TODO normals
#[cfg_attr(rustfmt, rustfmt_skip)]
const CUBE: [Vertex; 8] = [
    // front face
    Vertex { a_Pos: [ -1.0, 1.0, -1.0 ] },
    Vertex { a_Pos: [ 1.0, 1.0, -1.0 ] },
    Vertex { a_Pos: [ 1.0, -1.0, -1.0 ] },
    Vertex { a_Pos: [ -1.0, -1.0, -1.0 ] },

    // back face
    Vertex { a_Pos: [ -1.0, 1.0, 1.0 ] },
    Vertex { a_Pos: [ 1.0, 1.0, 1.0 ] },
    Vertex { a_Pos: [ 1.0, -1.0, 1.0 ] },
    Vertex { a_Pos: [ -1.0, -1.0, 1.0 ] },
];

/// Left-handed, +Y up, clockwise
#[cfg_attr(rustfmt, rustfmt_skip)]
const CUBE_INDICES: [u16; 36] = [
    0, 1, 2, 3, 0, 2, // front face
    4, 5, 1, 0, 4, 1, // top face
    4, 0, 3, 7, 4, 3, // left face
    1, 5, 6, 2, 1, 6, // right face
    3, 2, 6, 7, 3, 6, // bottom face
    5, 4, 7, 6, 5, 7, // back face
];

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_log::init_with_level(log::Level::Debug).unwrap();

    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    #[cfg(not(any(
        feature = "vulkan",
        feature = "dx11",
        feature = "dx12",
        feature = "metal",
        feature = "gl",
    )))]
    eprintln!(
        "You are running the example with the empty backend, no graphical output is to be expected"
    );

    let teapot = wavefront_obj::obj::parse(include_str!("./data/teapot.obj")).unwrap();
    assert_eq!(teapot.objects.len(), 1);
    let teapot_vertices = teapot.objects[0]
        .vertices
        .iter()
        .map(|vertex| Vertex {
            a_Pos: [vertex.x as f32, vertex.y as f32, vertex.z as f32],
        })
        .collect::<Vec<_>>();
    let teapot_indices = {
        let object = &teapot.objects[0];
        assert_eq!(object.geometry.len(), 1);
        object.geometry[0]
            .shapes
            .iter()
            .flat_map(|shape| match shape.primitive {
                wavefront_obj::obj::Primitive::Point(_) => {
                    unimplemented!()
                }
                wavefront_obj::obj::Primitive::Line(_, _) => {
                    unimplemented!()
                }
                wavefront_obj::obj::Primitive::Triangle(a, b, c) => std::iter::once(a.0 as u16)
                    .chain(std::iter::once(b.0 as u16))
                    .chain(std::iter::once(c.0 as u16)),
            })
            .collect::<Vec<_>>()
    };

    let event_loop = winit::event_loop::EventLoop::new();

    let wb = winit::window::WindowBuilder::new()
        .with_min_inner_size(winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(
            64.0, 64.0,
        )))
        .with_inner_size(winit::dpi::Size::Physical(winit::dpi::PhysicalSize::new(
            DIMS.width,
            DIMS.height,
        )))
        .with_title("ray-tracing".to_string());

    // instantiate backend
    let window = wb.build(&event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .body()
        .unwrap()
        .append_child(&winit::platform::web::WindowExtWebSys::canvas(&window))
        .unwrap();

    let instance =
        back::Instance::create("gfx-rs ray-tracing", 1).expect("Failed to create an instance!");

    let surface = unsafe {
        instance
            .create_surface(&window)
            .expect("Failed to create a surface!")
    };

    let mut adapters = instance.enumerate_adapters();

    for adapter in &adapters {
        println!("{:?}", adapter.info);
    }

    let adapter = adapters.remove(0);

    let required_features =
        hal::Features::ACCELERATION_STRUCTURE | hal::Features::RAY_TRACING_PIPELINE;

    assert!(adapter
        .physical_device
        .features()
        .contains(required_features));

    let memory_types = adapter.physical_device.memory_properties().memory_types;
    let limits = adapter.physical_device.limits();

    // Build a new device and associated command queues
    let family = adapter
        .queue_families
        .iter()
        .find(|family| {
            surface.supports_queue_family(family) && family.queue_type().supports_graphics()
        })
        .expect("No queue family supports presentation");
    let mut gpu = unsafe {
        adapter
            .physical_device
            .open(&[(family, &[1.0])], required_features)
            .unwrap()
    };
    let mut queue_group = gpu.queue_groups.pop().unwrap();
    let device = gpu.device;

    let mut command_pool = unsafe {
        device.create_command_pool(queue_group.family, pool::CommandPoolCreateFlags::empty())
    }
    .expect("Can't create command pool");

    let vertex_buffer = upload_to_buffer::<back::Backend, _>(
        &device,
        limits.non_coherent_atom_size as u64,
        &memory_types,
        buffer::Usage::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY
            | buffer::Usage::SHADER_DEVICE_ADDRESS,
        &teapot_vertices,
    );

    let index_buffer = upload_to_buffer::<back::Backend, _>(
        &device,
        limits.non_coherent_atom_size as u64,
        &memory_types,
        buffer::Usage::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY
            | buffer::Usage::SHADER_DEVICE_ADDRESS,
        &teapot_indices,
    );

    unsafe {
        let geometry_desc = accel::GeometryDesc {
            flags: accel::Flags::ALLOW_COMPACTION,
            ty: accel::Type::BottomLevel,
            geometries: &[&accel::Geometry {
                flags: accel::GeometryFlags::OPAQUE,
                geometry: accel::GeometryData::Triangles(accel::GeometryTriangles {
                    vertex_format: format::Format::Rgb32Sfloat,
                    vertex_buffer: &vertex_buffer.0,
                    vertex_buffer_offset: 0,
                    vertex_buffer_stride: std::mem::size_of::<Vertex>() as u32,
                    max_vertex: teapot_vertices.len() as u64,
                    index_buffer: Some((&index_buffer.0, 0, IndexType::U16)),
                    transform: None,
                }),
            }],
        };

        let teapot_primitive_count = (teapot_indices.len() / 3) as u32;
        let teapot_blas_requirements = device.get_acceleration_structure_build_requirements(
            &geometry_desc,
            &[teapot_primitive_count],
        );

        let raw_size = teapot_vertices.len() * mem::size_of::<Vertex>()
            + teapot_vertices.len() * mem::size_of::<u16>();
        dbg!(teapot_vertices.len());
        dbg!(teapot_indices.len());
        dbg!(raw_size);
        dbg!(&teapot_blas_requirements);
        dbg!(teapot_blas_requirements.acceleration_structure_size as f64 / raw_size as f64);
        dbg!(teapot_blas_requirements.build_scratch_size as f64 / raw_size as f64);
        assert!(teapot_blas_requirements.acceleration_structure_size > 0);
        assert_eq!(teapot_blas_requirements.update_scratch_size, 0);
        assert!(teapot_blas_requirements.build_scratch_size > 0);

        let scratch_buffer = create_empty_buffer::<back::Backend>(
            &device,
            limits.non_coherent_atom_size as u64,
            &memory_types,
            buffer::Usage::ACCELERATION_STRUCTURE_STORAGE | buffer::Usage::SHADER_DEVICE_ADDRESS,
            teapot_blas_requirements.build_scratch_size,
        );

        let accel_struct_bottom_buffer = create_empty_buffer::<back::Backend>(
            &device,
            limits.non_coherent_atom_size as u64,
            &memory_types,
            buffer::Usage::ACCELERATION_STRUCTURE_STORAGE | buffer::Usage::SHADER_DEVICE_ADDRESS,
            teapot_blas_requirements.acceleration_structure_size,
        );

        let mut teapot_blas = AccelerationStructure::<back::Backend> {
            accel_struct: device
                .create_acceleration_structure(&accel::CreateDesc {
                    buffer: &accel_struct_bottom_buffer.0,
                    buffer_offset: 0,
                    size: teapot_blas_requirements.acceleration_structure_size,
                    ty: accel::Type::BottomLevel,
                })
                .unwrap(),
            backing: accel_struct_bottom_buffer,
        };

        dbg!(&teapot_blas);

        device.set_acceleration_structure_name(&mut teapot_blas.accel_struct, "teapot");

        {
            // build the blas + get the compacted size

            let query_count = 1;
            let compacted_size_pool = device
                .create_query_pool(
                    hal::query::Type::AccelerationStructureCompactedSize,
                    query_count,
                )
                .unwrap();
            let serialized_size_pool = device
                .create_query_pool(
                    hal::query::Type::AccelerationStructureSerializationSize,
                    query_count,
                )
                .unwrap();

            let mut build_fence = device.create_fence(false).unwrap();
            let mut cmd_buffer = command_pool.allocate_one(command::Level::Primary);
            cmd_buffer.begin_primary(command::CommandBufferFlags::ONE_TIME_SUBMIT);

            cmd_buffer.build_acceleration_structures(&[(
                &accel::BuildDesc {
                    src: None,
                    dst: &teapot_blas.accel_struct,
                    geometry: &geometry_desc,
                    scratch: &scratch_buffer.0,
                    scratch_offset: 0,
                },
                &[accel::BuildRangeDesc {
                    primitive_count: teapot_primitive_count,
                    primitive_offset: 0,
                    first_vertex: 0,
                    transform_offset: 0,
                }][..],
            )]);

            cmd_buffer.pipeline_barrier(
                pso::PipelineStage::ACCELERATION_STRUCTURE_BUILD
                    ..pso::PipelineStage::ACCELERATION_STRUCTURE_BUILD,
                memory::Dependencies::empty(),
                &[memory::Barrier::AllBuffers(
                    buffer::Access::ACCELERATION_STRUCTURE_WRITE
                        ..buffer::Access::ACCELERATION_STRUCTURE_READ,
                )],
            );

            cmd_buffer.write_acceleration_structures_properties(
                &[&teapot_blas.accel_struct],
                hal::query::Type::AccelerationStructureCompactedSize,
                &compacted_size_pool,
                0,
            );

            cmd_buffer.write_acceleration_structures_properties(
                &[&teapot_blas.accel_struct],
                hal::query::Type::AccelerationStructureSerializationSize,
                &serialized_size_pool,
                0,
            );

            cmd_buffer.finish();

            queue_group.queues[0]
                .submit_without_semaphores(Some(&cmd_buffer), Some(&mut build_fence));

            device
                .wait_for_fence(&build_fence, !0)
                .expect("Can't wait for fence");

            let mut data = std::iter::repeat(0)
                .take(1 * mem::size_of::<u32>())
                .collect::<Vec<_>>();
            device
                .get_query_pool_results(
                    &compacted_size_pool,
                    0..query_count,
                    data.as_mut_slice(),
                    mem::size_of::<u32>() as hal::buffer::Stride,
                    hal::query::ResultFlags::WAIT,
                )
                .unwrap();
            let teapot_blas_compacted_size = (data.as_ptr() as *const u32).read() as u64;
            dbg!(
                teapot_blas_compacted_size,
                teapot_blas_requirements.acceleration_structure_size - teapot_blas_compacted_size,
                teapot_blas_compacted_size as f64
                    / teapot_blas_requirements.acceleration_structure_size as f64
            );

            let mut data = std::iter::repeat(0)
                .take(1 * mem::size_of::<u32>())
                .collect::<Vec<_>>();
            device
                .get_query_pool_results(
                    &serialized_size_pool,
                    0..query_count,
                    data.as_mut_slice(),
                    mem::size_of::<u32>() as hal::buffer::Stride,
                    hal::query::ResultFlags::WAIT,
                )
                .unwrap();
            let teapot_blas_serialized_size = (data.as_ptr() as *const u32).read();
            dbg!(teapot_blas_serialized_size);

            {
                // compact it!

                let accel_struct_bottom_buffer_compact = create_empty_buffer::<back::Backend>(
                    &device,
                    limits.non_coherent_atom_size as u64,
                    &memory_types,
                    buffer::Usage::ACCELERATION_STRUCTURE_STORAGE
                        | buffer::Usage::SHADER_DEVICE_ADDRESS,
                    teapot_blas_compacted_size,
                );
                let mut teapot_blas_compact = AccelerationStructure::<back::Backend> {
                    accel_struct: device
                        .create_acceleration_structure(&accel::CreateDesc {
                            buffer: &accel_struct_bottom_buffer_compact.0,
                            buffer_offset: 0,
                            size: teapot_blas_compacted_size,
                            ty: accel::Type::BottomLevel,
                        })
                        .unwrap(),
                    backing: accel_struct_bottom_buffer_compact,
                };
                device.set_acceleration_structure_name(
                    &mut teapot_blas_compact.accel_struct,
                    "teapot_compact",
                );

                let mut build_fence = device.create_fence(false).unwrap();
                let mut cmd_buffer = command_pool.allocate_one(command::Level::Primary);
                cmd_buffer.begin_primary(command::CommandBufferFlags::ONE_TIME_SUBMIT);

                cmd_buffer.copy_acceleration_structure(
                    &teapot_blas.accel_struct,
                    &teapot_blas_compact.accel_struct,
                    accel::CopyMode::Compact,
                );

                cmd_buffer.finish();

                queue_group.queues[0]
                    .submit_without_semaphores(Some(&cmd_buffer), Some(&mut build_fence));

                device
                    .wait_for_fence(&build_fence, !0)
                    .expect("Can't wait for fence");

                let _ = mem::replace(&mut teapot_blas, teapot_blas_compact);
            }
        }

        let instances = [{
            let mut instance = accel::Instance::new(
                device.get_acceleration_structure_address(&teapot_blas.accel_struct),
            );
            // instance.set_flags(accel::InstanceFlags::FORCE_OPAQUE);
            instance
        }];

        dbg!(&instances);

        let instances_buffer = upload_to_buffer::<back::Backend, _>(
            &device,
            limits.non_coherent_atom_size as u64,
            &memory_types,
            buffer::Usage::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY
                | buffer::Usage::SHADER_DEVICE_ADDRESS,
            &instances,
        );

        let top_level_geometry_desc = accel::GeometryDesc {
            flags: accel::Flags::ALLOW_COMPACTION,
            ty: accel::Type::TopLevel,
            geometries: &[&accel::Geometry {
                flags: accel::GeometryFlags::OPAQUE,
                geometry: accel::GeometryData::Instances(accel::GeometryInstances {
                    buffer: &instances_buffer.0,
                    buffer_offset: 0,
                }),
            }],
        };

        let tlas_requirements =
            device.get_acceleration_structure_build_requirements(&top_level_geometry_desc, &[1]);
        dbg!(&tlas_requirements);

        let tlas_scratch_buffer = create_empty_buffer::<back::Backend>(
            &device,
            limits.non_coherent_atom_size as u64,
            &memory_types,
            buffer::Usage::ACCELERATION_STRUCTURE_STORAGE | buffer::Usage::SHADER_DEVICE_ADDRESS,
            teapot_blas_requirements.build_scratch_size,
        );

        let tlas_buffer = create_empty_buffer::<back::Backend>(
            &device,
            limits.non_coherent_atom_size as u64,
            &memory_types,
            buffer::Usage::ACCELERATION_STRUCTURE_STORAGE | buffer::Usage::SHADER_DEVICE_ADDRESS,
            teapot_blas_requirements.acceleration_structure_size,
        );

        let mut tlas = AccelerationStructure::<back::Backend> {
            accel_struct: device
                .create_acceleration_structure(&accel::CreateDesc {
                    buffer: &tlas_buffer.0,
                    buffer_offset: 0,
                    size: tlas_requirements.acceleration_structure_size,
                    ty: accel::Type::TopLevel,
                })
                .unwrap(),
            backing: tlas_buffer,
        };

        device.set_acceleration_structure_name(&mut tlas.accel_struct, "tlas");

        {
            let query_count = 1;
            let compacted_size_pool = device
                .create_query_pool(
                    hal::query::Type::AccelerationStructureCompactedSize,
                    query_count,
                )
                .unwrap();
            let serialized_size_pool = device
                .create_query_pool(
                    hal::query::Type::AccelerationStructureSerializationSize,
                    query_count,
                )
                .unwrap();

            let mut build_fence = device.create_fence(false).unwrap();
            let mut cmd_buffer = command_pool.allocate_one(command::Level::Primary);
            cmd_buffer.begin_primary(command::CommandBufferFlags::ONE_TIME_SUBMIT);

            cmd_buffer.build_acceleration_structures(&[(
                &accel::BuildDesc {
                    src: None,
                    dst: &tlas.accel_struct,
                    geometry: &top_level_geometry_desc,
                    scratch: &tlas_scratch_buffer.0,
                    scratch_offset: 0,
                },
                &[accel::BuildRangeDesc {
                    primitive_count: 1,
                    primitive_offset: 0,
                    first_vertex: 0,
                    transform_offset: 0,
                }][..],
            )]);

            cmd_buffer.pipeline_barrier(
                pso::PipelineStage::ACCELERATION_STRUCTURE_BUILD
                    ..pso::PipelineStage::ACCELERATION_STRUCTURE_BUILD,
                memory::Dependencies::empty(),
                &[memory::Barrier::AllBuffers(
                    buffer::Access::ACCELERATION_STRUCTURE_WRITE
                        ..buffer::Access::ACCELERATION_STRUCTURE_READ,
                )],
            );

            cmd_buffer.write_acceleration_structures_properties(
                &[&tlas.accel_struct],
                hal::query::Type::AccelerationStructureCompactedSize,
                &compacted_size_pool,
                0,
            );

            cmd_buffer.write_acceleration_structures_properties(
                &[&tlas.accel_struct],
                hal::query::Type::AccelerationStructureSerializationSize,
                &serialized_size_pool,
                0,
            );

            cmd_buffer.finish();

            queue_group.queues[0]
                .submit_without_semaphores(Some(&cmd_buffer), Some(&mut build_fence));

            device
                .wait_for_fence(&build_fence, !0)
                .expect("Can't wait for fence");

            let mut data = std::iter::repeat(0)
                .take(1 * mem::size_of::<u32>())
                .collect::<Vec<_>>();
            device
                .get_query_pool_results(
                    &compacted_size_pool,
                    0..query_count,
                    data.as_mut_slice(),
                    mem::size_of::<u32>() as hal::buffer::Stride,
                    hal::query::ResultFlags::WAIT,
                )
                .unwrap();
            let tlas_compacted_size = (data.as_ptr() as *const u32).read();
            dbg!(tlas_compacted_size);

            let mut data = std::iter::repeat(0)
                .take(1 * mem::size_of::<u32>())
                .collect::<Vec<_>>();
            device
                .get_query_pool_results(
                    &serialized_size_pool,
                    0..query_count,
                    data.as_mut_slice(),
                    mem::size_of::<u32>() as hal::buffer::Stride,
                    hal::query::ResultFlags::WAIT,
                )
                .unwrap();
            let tlas_serialized_size = (data.as_ptr() as *const u32).read();
            dbg!(tlas_serialized_size);
        }

        {
            // do a dummy descriptor write

            let mut descriptor_pool = device
                .create_descriptor_pool(
                    1,
                    &[pso::DescriptorRangeDesc {
                        ty: pso::DescriptorType::AccelerationStructure,
                        count: 1,
                    }],
                    pso::DescriptorPoolCreateFlags::empty(),
                )
                .unwrap();

            let layout = device
                .create_descriptor_set_layout(
                    &[pso::DescriptorSetLayoutBinding {
                        binding: 0,
                        ty: pso::DescriptorType::AccelerationStructure,
                        count: 1,
                        stage_flags: pso::ShaderStageFlags::ALL,
                        immutable_samplers: false,
                    }],
                    &[],
                )
                .unwrap();
            let descriptor_set = descriptor_pool.allocate_set(&layout).unwrap();

            device.write_descriptor_sets(iter::once(pso::DescriptorSetWrite {
                set: &descriptor_set,
                binding: 0,
                array_offset: 0,
                descriptors: vec![pso::Descriptor::AccelerationStructure(&tlas.accel_struct)],
            }));
        }
    }
}

#[derive(Debug)]
struct AccelerationStructure<B: hal::Backend> {
    accel_struct: B::AccelerationStructure,
    backing: (B::Buffer, B::Memory),
}

fn create_empty_buffer<B: hal::Backend>(
    device: &B::Device,
    non_coherent_alignment: u64,
    memory_types: &[adapter::MemoryType],
    usage: buffer::Usage,
    size: u64,
) -> (B::Buffer, B::Memory) {
    let buffer_len = size;
    assert_ne!(buffer_len, 0);
    let padded_buffer_len = ((buffer_len + non_coherent_alignment - 1) / non_coherent_alignment)
        * non_coherent_alignment;

    let mut buffer = unsafe { device.create_buffer(padded_buffer_len, usage) }.unwrap();

    let buffer_req = unsafe { device.get_buffer_requirements(&buffer) };

    let upload_type = memory_types
        .iter()
        .enumerate()
        .position(|(id, mem_type)| {
            // type_mask is a bit field where each bit represents a memory type. If the bit is set
            // to 1 it means we can use that type for our buffer. So this code finds the first
            // memory type that has a `1` (or, is allowed), and is visible to the CPU.
            buffer_req.type_mask & (1 << id) != 0
                && mem_type
                    .properties
                    .contains(memory::Properties::CPU_VISIBLE)
        })
        .unwrap()
        .into();

    // TODO: check transitions: read/write mapping and buffer read
    let buffer_memory = unsafe {
        let memory = device
            .allocate_memory(upload_type, buffer_req.size)
            .unwrap();
        device.bind_buffer_memory(&memory, 0, &mut buffer).unwrap();
        memory
    };

    (buffer, buffer_memory)
}

fn upload_to_buffer<B: hal::Backend, T>(
    device: &B::Device,
    non_coherent_alignment: u64,
    memory_types: &[adapter::MemoryType],
    usage: buffer::Usage,
    data: &[T],
) -> (B::Buffer, B::Memory) {
    let buffer_stride = mem::size_of::<T>() as u64;
    let buffer_len = data.len() as u64 * buffer_stride;

    let (buffer, buffer_memory) = create_empty_buffer::<B>(
        device,
        non_coherent_alignment,
        memory_types,
        usage,
        buffer_len,
    );

    unsafe {
        let mapping = device
            .map_memory(&buffer_memory, memory::Segment::ALL)
            .unwrap();
        ptr::copy_nonoverlapping(data.as_ptr() as *const u8, mapping, buffer_len as usize);
        device
            .flush_mapped_memory_ranges(iter::once((&buffer_memory, memory::Segment::ALL)))
            .unwrap();
        device.unmap_memory(&buffer_memory);
    }

    (buffer, buffer_memory)
}
