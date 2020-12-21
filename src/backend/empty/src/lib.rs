//! Mock backend implementation to test the code for compile errors
//! outside of the graphics development environment.

extern crate gfx_hal as hal;

use crate::{
    buffer::Buffer,
    descriptor::{DescriptorPool, DescriptorSet, DescriptorSetLayout},
    image::Image,
    memory::Memory,
};

use hal::{adapter, command, device, format, pass, pool, pso, query, queue, window};
use log::debug;

use std::{borrow::Borrow, ops::Range};

mod buffer;
mod descriptor;
mod image;
mod memory;

const NOT_SUPPORTED_MESSAGE: &str = "This function is not currently mocked by the empty backend";

/// Dummy backend.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Backend {}
impl hal::Backend for Backend {
    type Instance = Instance;
    type PhysicalDevice = PhysicalDevice;
    type Device = Device;
    type Surface = Surface;

    type QueueFamily = QueueFamily;
    type CommandQueue = CommandQueue;
    type CommandBuffer = CommandBuffer;

    type Memory = Memory;
    type CommandPool = CommandPool;

    type ShaderModule = ();
    type RenderPass = ();
    type Framebuffer = ();

    type Buffer = Buffer;
    type BufferView = ();
    type Image = Image;
    type ImageView = ();
    type Sampler = ();

    type ComputePipeline = ();
    type GraphicsPipeline = ();
    type PipelineCache = ();
    type PipelineLayout = ();
    type DescriptorSetLayout = DescriptorSetLayout;
    type DescriptorPool = DescriptorPool;
    type DescriptorSet = DescriptorSet;

    type Fence = ();
    type Semaphore = ();
    type Event = ();
    type QueryPool = ();

    type AccelerationStructure = ();
}

/// Dummy physical device.
#[derive(Debug)]
pub struct PhysicalDevice;
impl adapter::PhysicalDevice<Backend> for PhysicalDevice {
    unsafe fn open(
        &self,
        families: &[(&QueueFamily, &[queue::QueuePriority])],
        _requested_features: hal::Features,
    ) -> Result<adapter::Gpu<Backend>, device::CreationError> {
        // Validate the arguments
        assert_eq!(
            families.len(),
            1,
            "Empty backend doesn't have multiple queue families"
        );
        let (_family, priorities) = families[0];
        assert_eq!(
            priorities.len(),
            1,
            "Empty backend doesn't support multiple queues"
        );
        let priority = priorities[0];
        assert!(
            0.0 <= priority && priority <= 1.0,
            "Queue priority is out of range"
        );

        // Create the queues
        let queue_groups = {
            let mut queue_group = queue::QueueGroup::new(QUEUE_FAMILY_ID);
            queue_group.add_queue(CommandQueue);
            vec![queue_group]
        };
        let gpu = adapter::Gpu {
            device: Device,
            queue_groups,
        };
        Ok(gpu)
    }

    fn format_properties(&self, _: Option<format::Format>) -> format::Properties {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    fn image_format_properties(
        &self,
        _: format::Format,
        _dim: u8,
        _: hal::image::Tiling,
        _: hal::image::Usage,
        _: hal::image::ViewCapabilities,
    ) -> Option<hal::image::FormatProperties> {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    fn memory_properties(&self) -> adapter::MemoryProperties {
        let memory_types = {
            use hal::memory::Properties;
            let properties = Properties::DEVICE_LOCAL
                | Properties::CPU_VISIBLE
                | Properties::COHERENT
                | Properties::CPU_CACHED;
            let memory_type = adapter::MemoryType {
                properties,
                heap_index: 0,
            };
            vec![memory_type]
        };
        // TODO: perhaps get an estimate of free RAM to report here?
        let memory_heaps = vec![adapter::MemoryHeap {
            size: 64 * 1024,
            flags: hal::memory::HeapFlags::empty(),
        }];
        adapter::MemoryProperties {
            memory_types,
            memory_heaps,
        }
    }

    fn features(&self) -> hal::Features {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    fn hints(&self) -> hal::Hints {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    fn limits(&self) -> hal::Limits {
        hal::Limits {
            non_coherent_atom_size: 1,
            optimal_buffer_copy_pitch_alignment: 1,
            ..Default::default()
        }
    }
}

/// Dummy command queue doing nothing.
#[derive(Debug)]
pub struct CommandQueue;
impl queue::CommandQueue<Backend> for CommandQueue {
    unsafe fn submit<'a, T, Ic, S, Iw, Is>(
        &mut self,
        _: queue::Submission<Ic, Iw, Is>,
        _: Option<&()>,
    ) where
        T: 'a + Borrow<CommandBuffer>,
        Ic: IntoIterator<Item = &'a T>,
        S: 'a + Borrow<()>,
        Iw: IntoIterator<Item = (&'a S, pso::PipelineStage)>,
        Is: IntoIterator<Item = &'a S>,
    {
    }

    unsafe fn present(
        &mut self,
        _surface: &mut Surface,
        _image: SwapchainImage,
        _wait_semaphore: Option<&()>,
    ) -> Result<Option<window::Suboptimal>, window::PresentError> {
        Ok(None)
    }

    fn wait_idle(&self) -> Result<(), device::OutOfMemory> {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }
}

/// Dummy device doing nothing.
#[derive(Debug)]
pub struct Device;
impl device::Device<Backend> for Device {
    unsafe fn create_command_pool(
        &self,
        _: queue::QueueFamilyId,
        _: pool::CommandPoolCreateFlags,
    ) -> Result<CommandPool, device::OutOfMemory> {
        Ok(CommandPool)
    }

    unsafe fn destroy_command_pool(&self, _: CommandPool) {}

    unsafe fn allocate_memory(
        &self,
        memory_type: hal::MemoryTypeId,
        size: u64,
    ) -> Result<Memory, device::AllocationError> {
        Memory::allocate(memory_type, size)
    }

    unsafe fn create_render_pass<'a, IA, IS, ID>(
        &self,
        _: IA,
        _: IS,
        _: ID,
    ) -> Result<(), device::OutOfMemory>
    where
        IA: IntoIterator,
        IA::Item: Borrow<pass::Attachment>,
        IS: IntoIterator,
        IS::Item: Borrow<pass::SubpassDesc<'a>>,
        ID: IntoIterator,
        ID::Item: Borrow<pass::SubpassDependency>,
    {
        Ok(())
    }

    unsafe fn create_pipeline_layout<IS, IR>(&self, _: IS, _: IR) -> Result<(), device::OutOfMemory>
    where
        IS: IntoIterator,
        IS::Item: Borrow<DescriptorSetLayout>,
        IR: IntoIterator,
        IR::Item: Borrow<(pso::ShaderStageFlags, Range<u32>)>,
    {
        Ok(())
    }

    unsafe fn create_pipeline_cache(
        &self,
        _data: Option<&[u8]>,
    ) -> Result<(), device::OutOfMemory> {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn get_pipeline_cache_data(&self, _cache: &()) -> Result<Vec<u8>, device::OutOfMemory> {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn destroy_pipeline_cache(&self, _: ()) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn create_graphics_pipeline<'a>(
        &self,
        _: &pso::GraphicsPipelineDesc<'a, Backend>,
        _: Option<&()>,
    ) -> Result<(), pso::CreationError> {
        Ok(())
    }

    unsafe fn create_compute_pipeline<'a>(
        &self,
        _: &pso::ComputePipelineDesc<'a, Backend>,
        _: Option<&()>,
    ) -> Result<(), pso::CreationError> {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn merge_pipeline_caches<I>(&self, _: &(), _: I) -> Result<(), device::OutOfMemory>
    where
        I: IntoIterator,
        I::Item: Borrow<()>,
    {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn create_framebuffer<I>(
        &self,
        _: &(),
        _: I,
        _: hal::image::Extent,
    ) -> Result<(), device::OutOfMemory>
    where
        I: IntoIterator,
        I::Item: Borrow<()>,
    {
        Ok(())
    }

    unsafe fn create_shader_module(&self, _: &[u32]) -> Result<(), device::ShaderError> {
        Ok(())
    }

    unsafe fn create_sampler(
        &self,
        _: &hal::image::SamplerDesc,
    ) -> Result<(), device::AllocationError> {
        Ok(())
    }

    unsafe fn create_buffer(
        &self,
        size: u64,
        _: hal::buffer::Usage,
    ) -> Result<Buffer, hal::buffer::CreationError> {
        Ok(Buffer::new(size))
    }

    unsafe fn get_buffer_requirements(&self, buffer: &Buffer) -> hal::memory::Requirements {
        hal::memory::Requirements {
            size: buffer.size,
            // TODO: perhaps require stronger alignments?
            alignment: 1,
            type_mask: !0,
        }
    }

    unsafe fn bind_buffer_memory(
        &self,
        _memory: &Memory,
        _: u64,
        _: &mut Buffer,
    ) -> Result<(), device::BindError> {
        Ok(())
    }

    unsafe fn create_buffer_view(
        &self,
        _: &Buffer,
        _: Option<format::Format>,
        _: hal::buffer::SubRange,
    ) -> Result<(), hal::buffer::ViewCreationError> {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn create_image(
        &self,
        kind: hal::image::Kind,
        _: hal::image::Level,
        _: format::Format,
        _: hal::image::Tiling,
        _: hal::image::Usage,
        _: hal::image::ViewCapabilities,
    ) -> Result<Image, hal::image::CreationError> {
        Ok(Image::new(kind))
    }

    unsafe fn get_image_requirements(&self, image: &Image) -> hal::memory::Requirements {
        image.get_requirements()
    }

    unsafe fn get_image_subresource_footprint(
        &self,
        _: &Image,
        _: hal::image::Subresource,
    ) -> hal::image::SubresourceFootprint {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn bind_image_memory(
        &self,
        _memory: &Memory,
        _: u64,
        _: &mut Image,
    ) -> Result<(), device::BindError> {
        Ok(())
    }

    unsafe fn create_image_view(
        &self,
        _: &Image,
        _: hal::image::ViewKind,
        _: format::Format,
        _: format::Swizzle,
        _: hal::image::SubresourceRange,
    ) -> Result<(), hal::image::ViewCreationError> {
        Ok(())
    }

    unsafe fn create_descriptor_pool<I>(
        &self,
        _: usize,
        _: I,
        _: pso::DescriptorPoolCreateFlags,
    ) -> Result<DescriptorPool, device::OutOfMemory>
    where
        I: IntoIterator,
        I::Item: Borrow<pso::DescriptorRangeDesc>,
    {
        Ok(DescriptorPool)
    }

    unsafe fn create_descriptor_set_layout<I, J>(
        &self,
        _bindings: I,
        _samplers: J,
    ) -> Result<DescriptorSetLayout, device::OutOfMemory>
    where
        I: IntoIterator,
        I::Item: Borrow<pso::DescriptorSetLayoutBinding>,
        J: IntoIterator,
        J::Item: Borrow<()>,
    {
        let layout = DescriptorSetLayout {
            name: String::new(),
        };
        Ok(layout)
    }

    unsafe fn write_descriptor_sets<'a, I, J>(&self, _: I)
    where
        I: IntoIterator<Item = pso::DescriptorSetWrite<'a, Backend, J>>,
        J: IntoIterator,
        J::Item: Borrow<pso::Descriptor<'a, Backend>>,
    {
    }

    unsafe fn copy_descriptor_sets<'a, I>(&self, _: I)
    where
        I: IntoIterator,
        I::Item: Borrow<pso::DescriptorSetCopy<'a, Backend>>,
    {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    fn create_semaphore(&self) -> Result<(), device::OutOfMemory> {
        Ok(())
    }

    fn create_fence(&self, _: bool) -> Result<(), device::OutOfMemory> {
        Ok(())
    }

    unsafe fn get_fence_status(&self, _: &()) -> Result<bool, device::DeviceLost> {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    fn create_event(&self) -> Result<(), device::OutOfMemory> {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn get_event_status(&self, _: &()) -> Result<bool, device::WaitError> {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_event(&self, _: &()) -> Result<(), device::OutOfMemory> {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn reset_event(&self, _: &()) -> Result<(), device::OutOfMemory> {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn create_query_pool(&self, _: query::Type, _: u32) -> Result<(), query::CreationError> {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn destroy_query_pool(&self, _: ()) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn get_query_pool_results(
        &self,
        _: &(),
        _: Range<query::Id>,
        _: &mut [u8],
        _: hal::buffer::Stride,
        _: query::ResultFlags,
    ) -> Result<bool, device::WaitError> {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn create_acceleration_structure(
        &self,
        _desc: &hal::acceleration_structure::CreateDesc<Backend>,
    ) -> Result<(), device::OutOfMemory> {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn destroy_acceleration_structure(&self, _acceleration_structure: ()) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn get_acceleration_structure_build_requirements(
        &self,
        _build_info: &hal::acceleration_structure::GeometryDesc<Backend>,
        _max_primitives_count: &[u32],
    ) -> hal::acceleration_structure::SizeRequirements {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn map_memory(
        &self,
        memory: &Memory,
        segment: hal::memory::Segment,
    ) -> Result<*mut u8, device::MapError> {
        memory.map(segment)
    }

    unsafe fn unmap_memory(&self, _memory: &Memory) {}

    unsafe fn flush_mapped_memory_ranges<'a, I>(&self, _: I) -> Result<(), device::OutOfMemory>
    where
        I: IntoIterator,
        I::Item: Borrow<(&'a Memory, hal::memory::Segment)>,
    {
        Ok(())
    }

    unsafe fn invalidate_mapped_memory_ranges<'a, I>(&self, _: I) -> Result<(), device::OutOfMemory>
    where
        I: IntoIterator,
        I::Item: Borrow<(&'a Memory, hal::memory::Segment)>,
    {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn free_memory(&self, _memory: Memory) {
        // Let memory drop
    }

    unsafe fn destroy_shader_module(&self, _: ()) {}

    unsafe fn destroy_render_pass(&self, _: ()) {}

    unsafe fn destroy_pipeline_layout(&self, _: ()) {}

    unsafe fn destroy_graphics_pipeline(&self, _: ()) {}

    unsafe fn destroy_compute_pipeline(&self, _: ()) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }
    unsafe fn destroy_framebuffer(&self, _: ()) {}

    unsafe fn destroy_buffer(&self, _: Buffer) {}

    unsafe fn destroy_buffer_view(&self, _: ()) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn destroy_image(&self, _: Image) {}

    unsafe fn destroy_image_view(&self, _: ()) {}

    unsafe fn destroy_sampler(&self, _: ()) {}

    unsafe fn destroy_descriptor_pool(&self, _: DescriptorPool) {}

    unsafe fn destroy_descriptor_set_layout(&self, _: DescriptorSetLayout) {}

    unsafe fn destroy_fence(&self, _: ()) {}

    unsafe fn destroy_semaphore(&self, _: ()) {}

    unsafe fn destroy_event(&self, _: ()) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    fn wait_idle(&self) -> Result<(), device::OutOfMemory> {
        Ok(())
    }

    unsafe fn set_image_name(&self, _: &mut Image, _: &str) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_buffer_name(&self, _: &mut Buffer, _: &str) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_command_buffer_name(&self, _: &mut CommandBuffer, _: &str) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_semaphore_name(&self, _: &mut (), _: &str) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_fence_name(&self, _: &mut (), _: &str) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_framebuffer_name(&self, _: &mut (), _: &str) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_render_pass_name(&self, _: &mut (), _: &str) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_descriptor_set_name(&self, set: &mut DescriptorSet, name: &str) {
        set.name = name.to_string();
    }

    unsafe fn set_descriptor_set_layout_name(&self, layout: &mut DescriptorSetLayout, name: &str) {
        layout.name = name.to_string();
    }

    unsafe fn set_pipeline_layout_name(&self, _pipeline_layout: &mut (), _name: &str) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_acceleration_structure_name(&self, _acceleration_structure: &mut (), name: &str) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn reset_fence(&self, _: &()) -> Result<(), device::OutOfMemory> {
        Ok(())
    }

    unsafe fn wait_for_fence(&self, _: &(), _: u64) -> Result<bool, device::WaitError> {
        Ok(true)
    }
}

#[derive(Debug)]
pub struct QueueFamily;
impl queue::QueueFamily for QueueFamily {
    fn queue_type(&self) -> queue::QueueType {
        queue::QueueType::General
    }
    fn max_queues(&self) -> usize {
        1
    }
    fn id(&self) -> queue::QueueFamilyId {
        QUEUE_FAMILY_ID
    }
}

const QUEUE_FAMILY_ID: queue::QueueFamilyId = queue::QueueFamilyId(0);

/// Dummy raw command pool.
#[derive(Debug)]
pub struct CommandPool;
impl pool::CommandPool<Backend> for CommandPool {
    unsafe fn allocate_one(&mut self, level: command::Level) -> CommandBuffer {
        assert_eq!(
            level,
            command::Level::Primary,
            "Only primary command buffers are supported"
        );
        CommandBuffer
    }

    unsafe fn reset(&mut self, _: bool) {}

    unsafe fn free<I>(&mut self, _: I)
    where
        I: IntoIterator<Item = CommandBuffer>,
    {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }
}

/// Dummy command buffer, which ignores all the calls.
#[derive(Debug)]
pub struct CommandBuffer;
impl command::CommandBuffer<Backend> for CommandBuffer {
    unsafe fn begin(
        &mut self,
        _: command::CommandBufferFlags,
        _: command::CommandBufferInheritanceInfo<Backend>,
    ) {
    }

    unsafe fn finish(&mut self) {}

    unsafe fn reset(&mut self, _: bool) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn pipeline_barrier<'a, T>(
        &mut self,
        _: Range<pso::PipelineStage>,
        _: hal::memory::Dependencies,
        _: T,
    ) where
        T: IntoIterator,
        T::Item: Borrow<hal::memory::Barrier<'a, Backend>>,
    {
    }

    unsafe fn fill_buffer(&mut self, _: &Buffer, _: hal::buffer::SubRange, _: u32) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn update_buffer(&mut self, _: &Buffer, _: hal::buffer::Offset, _: &[u8]) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn clear_image<T>(
        &mut self,
        _: &Image,
        _: hal::image::Layout,
        _: command::ClearValue,
        _: T,
    ) where
        T: IntoIterator,
        T::Item: Borrow<hal::image::SubresourceRange>,
    {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn clear_attachments<T, U>(&mut self, _: T, _: U)
    where
        T: IntoIterator,
        T::Item: Borrow<command::AttachmentClear>,
        U: IntoIterator,
        U::Item: Borrow<pso::ClearRect>,
    {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn resolve_image<T>(
        &mut self,
        _: &Image,
        _: hal::image::Layout,
        _: &Image,
        _: hal::image::Layout,
        _: T,
    ) where
        T: IntoIterator,
        T::Item: Borrow<command::ImageResolve>,
    {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn blit_image<T>(
        &mut self,
        _: &Image,
        _: hal::image::Layout,
        _: &Image,
        _: hal::image::Layout,
        _: hal::image::Filter,
        _: T,
    ) where
        T: IntoIterator,
        T::Item: Borrow<command::ImageBlit>,
    {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn bind_index_buffer(
        &mut self,
        _: &Buffer,
        _: hal::buffer::SubRange,
        _: hal::IndexType,
    ) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn bind_vertex_buffers<I, T>(&mut self, _: u32, _: I)
    where
        I: IntoIterator<Item = (T, hal::buffer::SubRange)>,
        T: Borrow<Buffer>,
    {
    }

    unsafe fn set_viewports<T>(&mut self, _: u32, _: T)
    where
        T: IntoIterator,
        T::Item: Borrow<pso::Viewport>,
    {
    }

    unsafe fn set_scissors<T>(&mut self, _: u32, _: T)
    where
        T: IntoIterator,
        T::Item: Borrow<pso::Rect>,
    {
    }

    unsafe fn set_stencil_reference(&mut self, _: pso::Face, _: pso::StencilValue) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_stencil_read_mask(&mut self, _: pso::Face, _: pso::StencilValue) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_stencil_write_mask(&mut self, _: pso::Face, _: pso::StencilValue) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_blend_constants(&mut self, _: pso::ColorValue) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_depth_bounds(&mut self, _: Range<f32>) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_line_width(&mut self, _: f32) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_depth_bias(&mut self, _: pso::DepthBias) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn begin_render_pass<T>(
        &mut self,
        _: &(),
        _: &(),
        _: pso::Rect,
        _: T,
        _: command::SubpassContents,
    ) where
        T: IntoIterator,
        T::Item: Borrow<command::ClearValue>,
    {
    }

    unsafe fn next_subpass(&mut self, _: command::SubpassContents) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn end_render_pass(&mut self) {}

    unsafe fn bind_graphics_pipeline(&mut self, _: &()) {}

    unsafe fn bind_graphics_descriptor_sets<I, J>(&mut self, _: &(), _: usize, _: I, _: J)
    where
        I: IntoIterator,
        I::Item: Borrow<DescriptorSet>,
        J: IntoIterator,
        J::Item: Borrow<command::DescriptorSetOffset>,
    {
        // Do nothing
    }

    unsafe fn bind_compute_pipeline(&mut self, _: &()) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn bind_compute_descriptor_sets<I, J>(&mut self, _: &(), _: usize, _: I, _: J)
    where
        I: IntoIterator,
        I::Item: Borrow<DescriptorSet>,
        J: IntoIterator,
        J::Item: Borrow<command::DescriptorSetOffset>,
    {
        // Do nothing
    }

    unsafe fn dispatch(&mut self, _: hal::WorkGroupCount) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn dispatch_indirect(&mut self, _: &Buffer, _: hal::buffer::Offset) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn copy_buffer<T>(&mut self, _: &Buffer, _: &Buffer, _: T)
    where
        T: IntoIterator,
        T::Item: Borrow<command::BufferCopy>,
    {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn copy_image<T>(
        &mut self,
        _: &Image,
        _: hal::image::Layout,
        _: &Image,
        _: hal::image::Layout,
        _: T,
    ) where
        T: IntoIterator,
        T::Item: Borrow<command::ImageCopy>,
    {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn copy_buffer_to_image<T>(&mut self, _: &Buffer, _: &Image, _: hal::image::Layout, _: T)
    where
        T: IntoIterator,
        T::Item: Borrow<command::BufferImageCopy>,
    {
    }

    unsafe fn copy_image_to_buffer<T>(&mut self, _: &Image, _: hal::image::Layout, _: &Buffer, _: T)
    where
        T: IntoIterator,
        T::Item: Borrow<command::BufferImageCopy>,
    {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn draw(&mut self, _: Range<hal::VertexCount>, _: Range<hal::InstanceCount>) {}

    unsafe fn draw_indexed(
        &mut self,
        _: Range<hal::IndexCount>,
        _: hal::VertexOffset,
        _: Range<hal::InstanceCount>,
    ) {
    }

    unsafe fn draw_indirect(
        &mut self,
        _: &Buffer,
        _: hal::buffer::Offset,
        _: hal::DrawCount,
        _: hal::buffer::Stride,
    ) {
    }

    unsafe fn draw_indexed_indirect(
        &mut self,
        _: &Buffer,
        _: hal::buffer::Offset,
        _: hal::DrawCount,
        _: hal::buffer::Stride,
    ) {
    }

    unsafe fn draw_indirect_count(
        &mut self,
        _: &Buffer,
        _: hal::buffer::Offset,
        _: &Buffer,
        _: hal::buffer::Offset,
        _: u32,
        _: hal::buffer::Stride,
    ) {
    }

    unsafe fn draw_indexed_indirect_count(
        &mut self,
        _: &Buffer,
        _: hal::buffer::Offset,
        _: &Buffer,
        _: hal::buffer::Offset,
        _: u32,
        _: hal::buffer::Stride,
    ) {
    }

    unsafe fn draw_mesh_tasks(&mut self, _: hal::TaskCount, _: hal::TaskCount) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn draw_mesh_tasks_indirect(
        &mut self,
        _: &Buffer,
        _: hal::buffer::Offset,
        _: hal::DrawCount,
        _: hal::buffer::Stride,
    ) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn draw_mesh_tasks_indirect_count(
        &mut self,
        _: &Buffer,
        _: hal::buffer::Offset,
        _: &Buffer,
        _: hal::buffer::Offset,
        _: u32,
        _: hal::buffer::Stride,
    ) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn set_event(&mut self, _: &(), _: pso::PipelineStage) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn reset_event(&mut self, _: &(), _: pso::PipelineStage) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn wait_events<'a, I, J>(&mut self, _: I, _: Range<pso::PipelineStage>, _: J)
    where
        I: IntoIterator,
        I::Item: Borrow<()>,
        J: IntoIterator,
        J::Item: Borrow<hal::memory::Barrier<'a, Backend>>,
    {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn begin_query(&mut self, _: query::Query<Backend>, _: query::ControlFlags) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn end_query(&mut self, _: query::Query<Backend>) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn reset_query_pool(&mut self, _: &(), _: Range<query::Id>) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn copy_query_pool_results(
        &mut self,
        _: &(),
        _: Range<query::Id>,
        _: &Buffer,
        _: hal::buffer::Offset,
        _: hal::buffer::Stride,
        _: query::ResultFlags,
    ) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn write_timestamp(&mut self, _: pso::PipelineStage, _: query::Query<Backend>) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn build_acceleration_structures<'a, I>(&self, descs: I)
    where
        I: IntoIterator<
            Item = (
                &'a hal::acceleration_structure::BuildDesc<'a, Backend>,
                // BuildRangeDesc array len must equal BuildDesc.geometry.geometries' len
                &'a [hal::acceleration_structure::BuildRangeDesc],
            ),
        >,
        I::IntoIter: ExactSizeIterator,
    {
        todo!()
    }

    unsafe fn build_acceleration_structures_indirect<'a, I>(&self, descs: I)
    where
        I: IntoIterator<
            Item = (
                &'a hal::acceleration_structure::BuildDesc<'a, Backend>,
                // `indirect_device_address` is a buffer device address that points to BuildDesc.geometry.geometries.len() BuildRangeDesc structures defining dynamic offsets to the addresses where geometry data is stored, as defined by BuildDesc.
                &'a Buffer,
                hal::buffer::Offset,
                hal::buffer::Offset, // stride
                // max_primitive_counts is an array of BuildDesc.geometry.geometries.len() values indicating the maximum number of primitives that will be built by this command for each geometry.
                &'a [u32],
            ),
        >,
        I::IntoIter: ExactSizeIterator,
    {
        todo!()
    }

    unsafe fn copy_acceleration_structure(
        &self,
        _src: &(),
        _dst: &(),
        _mode: hal::acceleration_structure::CopyMode,
    ) {
        todo!()
    }

    unsafe fn copy_acceleration_structure_to_memory(
        &self,
        _src: &(),
        _dst_buffer: &Buffer,
        _dst_offset: hal::buffer::Offset,
        _mode: hal::acceleration_structure::CopyMode,
    ) {
        todo!()
    }

    unsafe fn copy_memory_to_acceleration_structure(
        &self,
        _src_buffer: &Buffer,
        _src_offset: hal::buffer::Offset,
        _dst: &(),
        _mode: hal::acceleration_structure::CopyMode,
    ) {
        todo!()
    }

    unsafe fn write_acceleration_structures_properties(
        &self,
        _structures: &[&()],
        _query_type: query::Type,
        _pool: &(),
        _first_query: u32,
    ) {
        todo!()
    }

    unsafe fn push_graphics_constants(
        &mut self,
        _: &(),
        _: pso::ShaderStageFlags,
        _: u32,
        _: &[u32],
    ) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn push_compute_constants(&mut self, _: &(), _: u32, _: &[u32]) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn execute_commands<'a, T, I>(&mut self, _: I)
    where
        T: 'a + Borrow<CommandBuffer>,
        I: IntoIterator<Item = &'a T>,
    {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }

    unsafe fn insert_debug_marker(&mut self, _: &str, _: u32) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }
    unsafe fn begin_debug_marker(&mut self, _: &str, _: u32) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }
    unsafe fn end_debug_marker(&mut self) {
        unimplemented!("{}", NOT_SUPPORTED_MESSAGE)
    }
}

/// Dummy surface.
#[derive(Debug)]
pub struct Surface;
impl window::Surface<Backend> for Surface {
    fn supports_queue_family(&self, _: &QueueFamily) -> bool {
        true
    }

    fn capabilities(&self, _: &PhysicalDevice) -> window::SurfaceCapabilities {
        let extents = {
            let min_extent = window::Extent2D {
                width: 0,
                height: 0,
            };
            let max_extent = window::Extent2D {
                width: 8192,
                height: 4096,
            };
            min_extent..=max_extent
        };
        let usage = hal::image::Usage::COLOR_ATTACHMENT;
        let present_modes = window::PresentMode::all();
        let composite_alpha_modes = window::CompositeAlphaMode::OPAQUE;
        window::SurfaceCapabilities {
            image_count: 1..=1,
            current_extent: None,
            extents,
            max_image_layers: 1,
            usage,
            present_modes,
            composite_alpha_modes,
        }
    }

    fn supported_formats(&self, _: &PhysicalDevice) -> Option<Vec<format::Format>> {
        None
    }
}

#[derive(Debug)]
pub struct SwapchainImage;
impl Borrow<Image> for SwapchainImage {
    fn borrow(&self) -> &Image {
        unimplemented!()
    }
}
impl Borrow<()> for SwapchainImage {
    fn borrow(&self) -> &() {
        unimplemented!()
    }
}

impl window::PresentationSurface<Backend> for Surface {
    type SwapchainImage = SwapchainImage;

    unsafe fn configure_swapchain(
        &mut self,
        _: &Device,
        _: window::SwapchainConfig,
    ) -> Result<(), window::SwapchainError> {
        Ok(())
    }

    unsafe fn unconfigure_swapchain(&mut self, _: &Device) {}

    unsafe fn acquire_image(
        &mut self,
        _: u64,
    ) -> Result<(SwapchainImage, Option<window::Suboptimal>), window::AcquireError> {
        Ok((SwapchainImage, None))
    }
}

#[derive(Debug)]
pub struct Instance;

impl hal::Instance<Backend> for Instance {
    fn create(name: &str, version: u32) -> Result<Self, hal::UnsupportedBackend> {
        debug!(
            "Creating empty backend instance with name '{}' and version {}",
            name, version
        );
        Ok(Instance)
    }

    fn enumerate_adapters(&self) -> Vec<adapter::Adapter<Backend>> {
        // TODO: provide more mock adapters, with various qualities
        let info = adapter::AdapterInfo {
            name: "Mock Device".to_string(),
            vendor: 0,
            device: 1234,
            device_type: adapter::DeviceType::Other,
        };
        let adapter = adapter::Adapter {
            info,
            physical_device: PhysicalDevice,
            // TODO: multiple queue families
            queue_families: vec![QueueFamily],
        };
        vec![adapter]
    }

    unsafe fn create_surface(
        &self,
        raw_window_handle: &impl raw_window_handle::HasRawWindowHandle,
    ) -> Result<Surface, hal::window::InitError> {
        // TODO: maybe check somehow that the given handle is valid?
        let _handle = raw_window_handle.raw_window_handle();
        Ok(Surface)
    }

    unsafe fn destroy_surface(&self, _surface: Surface) {}
}
