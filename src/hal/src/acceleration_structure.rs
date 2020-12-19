//! TODO docs

// TODO remove and add docs
#![allow(missing_docs)]

use std::ops::Range;

use crate::{buffer::Offset, format::Format, Backend, IndexType};

/// Denotes the type of acceleration structure.
#[derive(Debug)]
pub enum AccelerationStructureType {
    ///
    TopLevel,
    ///
    BottomLevel,
    // TODO vulkan supports "generic level" (where the concrete build type is specified), but discourages its use for applications "written directly for Vulkan" since it "could affect capabilities or performance in the future" (https://www.khronos.org/blog/vulkan-ray-tracing-final-specification-release). Perhaps this is to better support `vkd3d-proton`, but we probably don't want it exposed in gfx?
}

/// A description of the data needed to build an acceleration structure.
#[derive(Debug)]
pub struct AccelerationStructureDesc<'a, B: Backend> {
    /// TODO: document lifetime required for this buffer
    pub buffer: &'a B::Buffer,
    /// TODO: the final gpu address on DX12 needs to be D3D12_RAYTRACING_ACCELERATION_STRUCTURE_BYTE_ALIGNMENT (256), but vulkan only requires this offset to be 256 (as defined by https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkAccelerationStructureCreateInfoKHR.html)
    pub buffer_offset: Offset,

    /// The type of acceleration structure to build.
    pub ty: AccelerationStructureType,
    // /// currently only has `accelerationStructureCaptureReplay`
    // create_flags: VkAccelerationStructureCreateFlagsKHR,
    // /// used for `accelerationStructureCaptureReplay`
    // device_address: VkDeviceAddress,
}

bitflags! {
    /// https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkBuildAccelerationStructureFlagBitsKHR.html
    pub struct AccelerationStructureFlags: u32 {
        /// indicates that the specified acceleration structure can be updated with update of VK_TRUE in vkCmdBuildAccelerationStructuresKHR or vkCmdBuildAccelerationStructureNV .
        const ALLOW_UPDATE = 0x1;
        ///  indicates that the specified acceleration structure can act as the source for a copy acceleration structure command with mode of VK_COPY_ACCELERATION_STRUCTURE_MODE_COMPACT_KHR to produce a compacted acceleration structure.
        const ALLOW_COMPACTION = 0x2;
        /// indicates that the given acceleration structure build should prioritize trace performance over build time.
        const PREFER_FAST_TRACE = 0x4;
        ///  indicates that the given acceleration structure build should prioritize build time over trace performance.
        const PREFER_FAST_BUILD = 0x8;
        /// indicates that this acceleration structure should minimize the size of the scratch memory and the final result build, potentially at the expense of build time or trace performance.
        const LOW_MEMORY = 0x10;
    }
}

/// TODO docs
#[derive(Debug)]
pub enum AccelerationStructureCreateMode {
    /// specifies that the destination acceleration structure will be built using the specified geometries.
    Build,
    /// specifies that the destination acceleration structure will be built using data in a source acceleration structure, updated by the specified geometries.
    /// Note there's constraints on update: https://microsoft.github.io/DirectX-Specs/d3d/Raytracing.html#acceleration-structure-update-constraints
    Update,
}

/// A description of the geometry data needed to populate an acceleration structure.
///
/// TODO: there's something here that smells w/ what fields are needed to get the required build size vs what fields are needed to actually build. Also, the top/bottom levels having different requirements on which fields are valid.
#[derive(Debug)]
pub struct AccelerationStructureGeometryDesc<'a, B: Backend> {
    pub flags: AccelerationStructureFlags,
    /// The type of acceleration structure to build.
    pub ty: AccelerationStructureType,
    pub mode: AccelerationStructureCreateMode,

    pub src: Option<B::AccelerationStructure>,
    pub dst: Option<B::AccelerationStructure>,

    // TODO: We could enforce the following the type system?
    // - in both vulkan (via `VUID-VkAccelerationStructureBuildGeometryInfoKHR-type-03792`) and DX12 (via type system), all of the structs here must be the same variant.
    // - blas must be triangles or aabbs, tlas must be instances
    pub geometries: &'a [AccelerationStructureGeometry<'a, B>],
    // Both APIs support "array" and "array of pointers", presumably to allow for cheap reuse of the geometry descriptors.
    // pgeometries: &'a [&'a GeometryDesc<'a, B>],
    pub scratch: Option<(&'a B::Buffer, Offset)>,
}

bitflags! {
    /// https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkGeometryFlagBitsKHR.html
    pub struct AccelerationStructureGeometryFlags: u32 {
        /// indicates that this geometry does not invoke the any-hit shaders even if present in a hit group.
        const OPAQUE = 0x1;
        /// indicates that the implementation must only call the any-hit shader a single time for each primitive in this geometry. If this bit is absent an implementation may invoke the any-hit shader more than once for this geometry.
        const NO_DUPLICATE_ANY_HIT_INVOCATION = 0x2;
    }
}

/// TODO docs
#[derive(Debug)]
pub struct AccelerationStructureGeometry<'a, B: Backend> {
    pub flags: AccelerationStructureGeometryFlags,
    pub geometry: Geometry<'a, B>,
}

/// TODO docs
#[derive(Debug)]
pub enum Geometry<'a, B: Backend> {
    /// TODO docs
    Triangles(GeometryTriangles<'a, B>),

    /// TODO docs
    /// TODO bikeshed capitalization of AABBs.
    Aabbs(GeometryAabbs<'a, B>),

    // TODO
    /// TODO docs
    Instances(GeometryInstances<'a, B>),
}

// https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkAccelerationStructureGeometryTrianglesDataKHR.html
// for docs: Memory Safety (https://nvpro-samples.github.io/vk_raytracing_tutorial_KHR/#accelerationstructure)
// TODO: I don't like that this is reused for the "get build sizes" and "do actual build" with complex rules on what fields are ignored when. Perhaps we could use DX12's model for `D3D12_BUILD_RAYTRACING_ACCELERATION_STRUCTURE_DESC` and `D3D12_BUILD_RAYTRACING_ACCELERATION_STRUCTURE_INPUTS`?
#[derive(Debug)]
pub struct GeometryTriangles<'a, B: Backend> {
    /// Both APIs require support for at least the following:
    /// - `(R32_G32, Float)`
    /// - `(R32_G32_B32, Float)`
    /// - `(R16_G16, Float)`
    /// - `(R16_G16_B16_A16, Float)`
    /// - `(R16_G16, Inorm)`
    /// - `(R16_G16_B16_A16, Inorm)`
    /// VK could support more by querying `VK_FORMAT_FEATURE_ACCELERATION_STRUCTURE_VERTEX_BUFFER_BIT_KHR`, DX12 is not queryable? Note [the DX12 ray tracing spec](https://microsoft.github.io/DirectX-Specs/d3d/Raytracing.html#d3d12_raytracing_geometry_triangles_desc) says it supports more than [the Win32 docs](https://docs.microsoft.com/en-us/windows/win32/api/d3d12/ns-d3d12-d3d12_raytracing_geometry_triangles_desc).
    pub vertex_format: Format,
    pub vertex_buffer: &'a B::Buffer,
    pub vertex_buffer_offset: Offset,
    pub vertex_buffer_stride: Offset,

    /// aka "vertex count"?
    pub max_vertex: Offset,

    // index format must be DXGI_FORMAT_R32_UINT, DXGI_FORMAT_R16_UINT
    // can also be VK_INDEX_TYPE_NONE_KHR/DXGI_FORMAT_UNKNOWN
    pub index_buffer: Option<(&'a B::Buffer, Offset, IndexType)>,

    /// 3x4 matrix
    /// TODO enum for cpu repr?
    /// Must point to an address containing `TransformMatrix`
    pub transform: Option<(&'a B::Buffer, Offset)>,
}

// VkTransformMatrixKHR
#[derive(Debug)]
pub struct TransformMatrix {
    /// 3x4 row-major affine transformation matrix. Use `mint::RowMatrix3x4` if available.
    pub matrix: [[f32; 4]; 3],
}

#[derive(Debug)]
pub struct GeometryAabbs<'a, B: Backend> {
    /// Must point to a buffer with buffer with an array of `AabbPositions`s
    /// TODO: document lifetime required for this buffer
    pub buffer: &'a B::Buffer,
    pub buffer_offset: Offset,
    pub buffer_stride: Offset,
}

// TODO doc
#[derive(Debug)]
pub struct AabbPositions {
    /// Use `mint::Vector3` if available.
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkAccelerationStructureGeometryInstancesDataKHR.html
#[derive(Debug)]
pub struct GeometryInstances<'a, B: Backend> {
    /// Must point to a buffer with buffer with an array of `Instance`s
    /// TODO: document lifetime required for this buffer
    /// TODO this struct also allows passing an array of pointers, idk if that makes sense outside the host operations case
    pub buffer: &'a B::Buffer,
    pub buffer_offset: Offset,
}

bitflags! {
    /// https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/VkGeometryInstanceFlagBitsKHR.html
    pub struct InstanceFlags: u32 {
        const TRIANGLE_FACING_CULL_DISABLE_BIT = 0x1;
        const TRIANGLE_FRONT_COUNTERCLOCKWISE_BIT = 0x2;
        const FORCE_OPAQUE_BIT = 0x4;
        const FORCE_NO_OPAQUE_BIT = 0x8;
    }
}

// TODO AFAIK rust doesn't have custom sized fields, so we'll need some binary writer wrapper to actually support this at an API level.
#[derive(Debug)]
pub struct Instance {
    transform: TransformMatrix,
    /// 24 bits
    instance_custom_index: u32,
    /// 8 bits visibility mask for the geometry. The instance may only be hit if rayMask & instance.mask != 0
    mask: u32,
    /// 24 bit
    instance_shader_binding_table_record_offset: u32,
    /// 8 bits
    flags: InstanceFlags,

    /// either B::AccelerationStructure (host operations) or GPU address (buffer + offset?)
    acceleration_structure_reference: u64,
}

#[derive(Debug)]
pub struct AccelerationStructureSizeRequirements {
    pub acceleration_structure_size: u64,
    pub update_scratch_size: u64,
    pub build_scratch_size: u64,
}

#[derive(Debug)]
pub enum AccelerationStructureCopyMode {
    Clone,
    Compact,

    // these are subject to the device supporting serialization--also (at least on DX12) mainly used for debug tooling and not load perf--I'm unsure what an end-user use case looks like.
    Serialize,
    Deserialize,
}

#[derive(Debug)]
pub struct AccelerationStructureBuildRangeDesc {
    pub primitive: Range<u32>,
    pub first_vertex: u32,
    /// "defines an offset in bytes into the memory where a transform matrix is defined.""
    /// TODO: this is documented as only used for `Geometry::Triangles`, but I have no idea why. GeometryTriangles::transform is implied to point to exactly one transform matrix, but I guess this pattern would allow for passing a list of transforms to reuse the same geometry (but not go through the TLAS' Instance::transform...). DX12 doesn't require any additional info at build time and thus doesn't have this concept--passing 0 here in the vulkan case would probably meet parity with DX12
    pub transform_offset: u32,
}
