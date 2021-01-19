//! TODO docs

use crate::{
    buffer::{Offset, Stride},
    format::Format,
    Backend, IndexType,
};

/// Denotes the type of acceleration structure.
#[derive(Debug, Copy, Clone)]
pub enum Type {
    /// A top-level acceleration structure containing [`GeometryData::Instances`] pointing to bottom-level acceleration structures.
    TopLevel,
    /// A bottom-level acceleration structure containing [`GeometryData::Triangles`] or [`GeometryData::Aabbs`].
    BottomLevel,
    /// An acceleration structure whose type is not known until build time. [`Self::TopLevel`] and [`Self::BottomLevel`] should be preferred over [`Self::Generic`].
    ///
    /// This is not valid during any of the acceleration structure build commands.
    Generic,
}

/// A description of the data needed to create an acceleration structure.
#[derive(Debug)]
pub struct CreateDesc<'a, B: Backend> {
    /// The buffer to store the acceleration structure in.
    pub buffer: &'a B::Buffer,

    /// The offset into `buffer` where the acceleration structure will be written. Must be a multiple of 256.
    pub buffer_offset: Offset,

    /// The size required for the acceleration structure.
    ///
    /// TODO additional notes on where to get this size for build vs compacting
    pub size: u64,

    /// The type of acceleration structure to build.
    pub ty: Type,
    // TODO(capture-replay)
    // /// currently only has `accelerationStructureCaptureReplay`
    // create_flags: VkAccelerationStructureCreateFlagsKHR,
    // /// used for `accelerationStructureCaptureReplay`
    // device_address: VkDeviceAddress,
}

/// A description of the data needed to build or update an acceleration structure with geometry data.
#[derive(Debug)]
pub struct BuildDesc<'a, B: Backend> {
    /// The original acceleration structure to base an update from.
    ///
    /// If `Some`, implies that we will do an update from `src` rather than a build from scratch.
    pub src: Option<&'a B::AccelerationStructure>,

    /// The acceleration structure to be built or updated.
    pub dst: &'a B::AccelerationStructure,

    /// The geometry data that will be written into this acceleration structure.
    pub geometry: &'a GeometryDesc<'a, B>,

    // TODO(cpu-repr)
    /// The buffer containing scratch space used to construct a acceleration structure.
    pub scratch: &'a B::Buffer,
    /// The offset into `scratch` which should be used for the scratch data.
    pub scratch_offset: Offset,
}

bitflags! {
    /// Option flags for acceleration structure builds.
    pub struct Flags: u32 {
        /// The acceleration structure can be updated during builds.
        const ALLOW_UPDATE = 0x1;
        /// The acceleration structure can be compacted during copies with [`CopyMode::Compact`].
        const ALLOW_COMPACTION = 0x2;
        /// The acceleration structure build should prioritize trace performance over build time.
        const PREFER_FAST_TRACE = 0x4;
        /// The acceleration structure build should prioritize trace build time over performance.
        const PREFER_FAST_BUILD = 0x8;
        /// The acceleration structure build should minimize scratch memory usage and final build size, potentially at the cost of build time or performance.
        const LOW_MEMORY = 0x10;
    }
}

/// A description of the geometry data needed to populate an acceleration structure.
#[derive(Debug)]
pub struct GeometryDesc<'a, B: Backend> {
    /// Acceleration structure build flags.
    pub flags: Flags,

    /// The type of acceleration structure to build.
    pub ty: Type,

    // TODO: We could enforce the following the type system?
    // - in both vulkan (via `VUID-VkAccelerationStructureBuildGeometryInfoKHR-type-03792`) and DX12 (via type system), all of the structs here must be the same variant.
    // - blas must be triangles or aabbs, tlas must be instances
    /// List of geometries to be stored in an acceleration structure.
    pub geometries: &'a [&'a Geometry<'a, B>],
}

bitflags! {
    /// Option flags for various acceleration structure geometry settings.
    pub struct GeometryFlags: u32 {
        /// This geometry will not invoke the any-hit shaders, even if present in a hit group.
        const OPAQUE = 0x1;
        /// The any-hit shader will only be called once per primitive in this geometry.
        const NO_DUPLICATE_ANY_HIT_INVOCATION = 0x2;
    }
}

/// Geometry data that can be used in an acceleration structure.
#[derive(Debug)]
pub struct Geometry<'a, B: Backend> {
    /// Flags to describe how this geometry will be intersected.
    pub flags: GeometryFlags,

    /// The data contained in this geometry.
    pub geometry: GeometryData<'a, B>,
}

///
#[derive(Debug)]
pub enum GeometryData<'a, B: Backend> {
    ///
    Triangles(GeometryTriangles<'a, B>),
    ///
    Aabbs(GeometryAabbs<'a, B>),
    ///
    Instances(GeometryInstances<'a, B>),
}

/// Geometry data containing triangle data.
#[derive(Debug)]
pub struct GeometryTriangles<'a, B: Backend> {
    // TODO: VK could support more by querying `VK_FORMAT_FEATURE_ACCELERATION_STRUCTURE_VERTEX_BUFFER_BIT_KHR`, DX12 is not queryable? Note [the DX12 ray tracing spec](https://microsoft.github.io/DirectX-Specs/d3d/Raytracing.html#d3d12_raytracing_geometry_triangles_desc) says it supports more than [the Win32 docs](https://docs.microsoft.com/en-us/windows/win32/api/d3d12/ns-d3d12-d3d12_raytracing_geometry_triangles_desc).
    /// The format of the vertex data in `vertex_buffer`.
    ///
    /// At least the following formats are supported:
    /// - `(R32_G32, Float)`: The Z component is implied to be 0.
    /// - `(R32_G32_B32, Float)`
    /// - `(R16_G16, Float)`: The Z component is implied to be 0.
    /// - `(R16_G16_B16_A16, Float)`: The A component is ignored.
    /// - `(R16_G16, Inorm)`: The Z component is implied to be 0.
    /// - `(R16_G16_B16_A16, Inorm)`: The A component is ignored.
    pub vertex_format: Format,

    // TODO(cpu-repr)
    /// The buffer containing the vertex data.
    pub vertex_buffer: &'a B::Buffer,
    /// The offset into `vertex_buffer` pointing to the start of the vertex data.
    pub vertex_buffer_offset: Offset,
    /// The space between vertices in `vertex_buffer`.
    pub vertex_buffer_stride: Stride,

    /// The index of the last vertex addressed by a build command using this geometry.
    pub max_vertex: Offset, // TODO vulkan api takes u32

    // TODO(cpu-repr)
    /// The buffer and offset containing the index data and the type of the indices.
    pub index_buffer: Option<(&'a B::Buffer, Offset, IndexType)>,

    /// TODO(cpu-repr)
    /// The buffer and offset containing a list of transform data.
    ///
    /// The buffer must contain a list of `TransformMatrix`.
    pub transform: Option<(&'a B::Buffer, Offset)>,
}

/// A 3x4 row-major affine transformation matrix.
// TODO `GeometryTriangles::transform` depends on the layout of this struct
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct TransformMatrix([[f32; 4]; 3]);

impl TransformMatrix {
    /// TODO docs
    pub fn identity() -> Self {
        Self([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ])
    }
}

/// Geometry data containing axis-aligned bounding box data.
#[derive(Debug)]
pub struct GeometryAabbs<'a, B: Backend> {
    // TODO(cpu-repr)
    /// The buffer containing the AABB data.
    ///
    /// The buffer must contain a list of `AabbPositions`.
    pub buffer: &'a B::Buffer,

    /// The offset into `buffer`.
    pub buffer_offset: Offset,

    /// The stride of the AABB data in `buffer`.
    pub buffer_stride: Stride,
}

/// An axis-aligned bounding box.
// TODO `GeometryAabbs::buffer` depends on the layout of this struct
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct AabbPositions {
    /// A 3D position containing the minimum corner of the AABB.
    pub min: [f32; 3],

    /// A 3D position containing the maximum corner of the AABB.
    pub max: [f32; 3],
}

/// Geometry data containing instance data.
#[derive(Debug)]
pub struct GeometryInstances<'a, B: Backend> {
    // TODO this struct also allows passing an array of pointers, idk if that makes sense outside the host operations case
    // TODO(cpu-repr)
    /// The buffer containing the instance data.
    ///
    /// The buffer must contain a list of `Instance`.
    pub buffer: &'a B::Buffer,

    /// The offset into `buffer`.
    pub buffer_offset: Offset,
}

bitflags! {
    /// Option flags for an acceleration structure instance.
    pub struct InstanceFlags: u8 {
        /// Disables face culling for this instance.
        const TRIANGLE_FACING_CULL_DISABLE = 0x1;
        /// Reverses front and back sides of geometry's triangles.
        ///
        /// Note the winding direction is calculated in object space, is not affected by instance transforms.
        const TRIANGLE_FRONT_COUNTERCLOCKWISE = 0x2;
        /// Override the `GeometryFlags` bottom-level acceleration structures to act as if `GeometryFlags::OPAQUE` was set.
        ///
        /// This flag can be overridden by the ray flags (TODO reference which flags when they are added)
        const FORCE_OPAQUE = 0x4;
        /// Override the `GeometryFlags` bottom-level acceleration structures to act as if `GeometryFlags::OPAQUE` was not set.
        ///
        /// This flag can be overridden by the ray flags (TODO reference which flags when they are added)
        const FORCE_NO_OPAQUE = 0x8;
    }
}

/// todo docs
///
/// pls do not by tempted by the inner value!
#[derive(Copy, Clone)]
#[repr(transparent)]
// TODO private ctor that backends have access to?
pub struct DeviceAddress(pub u64);

impl std::fmt::Debug for DeviceAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct DebugAsHex(u64);

        impl std::fmt::Debug for DebugAsHex {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::LowerHex::fmt(&self.0, f)
            }
        }

        f.debug_tuple("DeviceAddress")
            .field(&DebugAsHex(self.0))
            .finish()
    }
}

impl std::fmt::Pointer for DeviceAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::LowerHex::fmt(&self.0, f)
    }
}

/// An instance pointing to some bottom-level acceleration structure data.
///
/// Note: there are fields that are combined because driver APIs require this struct to have a specific layout and to be written, tightly packed, into a GPU buffer to be consumed. Consider using the helper methods on this type to assign to those fields.
#[derive(Clone)]
#[repr(C)]
pub struct Instance {
    /// The instance transform matrix that should be applied to the referenced acceleration structure.
    pub transform: TransformMatrix,

    /// Combined instance custom index and mask into a single field.
    /// - Top 24 bits are the custom index
    /// - Bottom 8 bits are the visibility mask for the geometry. The instance may only be hit if rayMask & instance.mask != 0
    pub instance_custom_index_24_and_mask_8: u32,

    /// Combined instance shader binding table record offset and flags into a single field.
    /// - Top 24 bits are the SBT record offset
    /// - Bottom 8 bits are `InstanceFlags`
    pub instance_shader_binding_table_record_offset_24_and_flags_8: u32,

    /// The bottom-level acceleration structure this `Instance` refers to.
    // TODO(host-commands): either B::AccelerationStructure (host commands)
    pub acceleration_structure_reference: DeviceAddress,
}

const TOP_24_MASK: u32 = 0xFFFFFF00;
const BOTTOM_8_MASK: u32 = 0xFF;

impl std::fmt::Debug for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Instance")
            .field("transform", &self.transform)
            .field(
                "instance_custom_index",
                &((self.instance_custom_index_24_and_mask_8 & TOP_24_MASK) >> 8),
            )
            .field(
                "mask",
                &(self.instance_custom_index_24_and_mask_8 & BOTTOM_8_MASK),
            )
            .field(
                "instance_shader_binding_table_record_offset",
                &((self.instance_shader_binding_table_record_offset_24_and_flags_8 & TOP_24_MASK)
                    >> 8),
            )
            .field(
                "flags",
                &(InstanceFlags::from_bits(
                    (self.instance_shader_binding_table_record_offset_24_and_flags_8
                        & BOTTOM_8_MASK) as u8,
                )
                .unwrap()),
            )
            .field(
                "acceleration_structure_reference",
                &self.acceleration_structure_reference,
            )
            .finish()
    }
}

// TODO tests
impl Instance {
    /// TODO docs
    pub fn new(blas: DeviceAddress) -> Self {
        Self {
            transform: TransformMatrix::identity(),
            instance_custom_index_24_and_mask_8: 0,
            instance_shader_binding_table_record_offset_24_and_flags_8: 0,
            acceleration_structure_reference: blas,
        }
    }

    fn fits_in_24_bits(n: u32) -> bool {
        n < 2 << 24
    }

    fn replace_bits(destination: u32, new_bits: u32, new_bits_mask: u32) -> u32 {
        destination ^ ((destination ^ new_bits) & new_bits_mask)
    }

    /// TODO docs
    pub fn set_instance_custom_index(&mut self, instance_custom_index: u32) {
        assert!(Self::fits_in_24_bits(instance_custom_index));
        self.instance_custom_index_24_and_mask_8 = Self::replace_bits(
            self.instance_custom_index_24_and_mask_8,
            instance_custom_index << 8,
            TOP_24_MASK,
        );
    }

    /// TODO docs
    pub fn set_mask(&mut self, mask: u8) {
        self.instance_custom_index_24_and_mask_8 = Self::replace_bits(
            self.instance_custom_index_24_and_mask_8,
            mask as u32,
            BOTTOM_8_MASK,
        );
    }

    /// TODO docs
    pub fn set_instance_shader_binding_table_record_offset(
        &mut self,
        instance_shader_binding_table_record_offset: u32,
    ) {
        assert!(Self::fits_in_24_bits(
            instance_shader_binding_table_record_offset
        ));
        self.instance_shader_binding_table_record_offset_24_and_flags_8 = Self::replace_bits(
            self.instance_shader_binding_table_record_offset_24_and_flags_8,
            instance_shader_binding_table_record_offset << 8,
            TOP_24_MASK,
        );
    }

    /// TODO docs
    pub fn set_flags(&mut self, flags: InstanceFlags) {
        self.instance_shader_binding_table_record_offset_24_and_flags_8 = Self::replace_bits(
            self.instance_shader_binding_table_record_offset_24_and_flags_8,
            flags.bits() as u32,
            BOTTOM_8_MASK,
        );
    }
}

#[cfg(test)]
mod struct_size_tests {
    use super::*;

    #[test]
    fn transform_matrix() {
        assert_eq!(std::mem::size_of::<TransformMatrix>(), 48);
        assert_eq!(std::mem::size_of::<[TransformMatrix; 2]>(), 96);
    }

    #[test]
    fn aabb_positions() {
        assert_eq!(std::mem::size_of::<AabbPositions>(), 24);
        assert_eq!(std::mem::size_of::<[AabbPositions; 2]>(), 48);
    }

    #[test]
    fn instance() {
        assert_eq!(std::mem::size_of::<Instance>(), 64);
        assert_eq!(std::mem::size_of::<[Instance; 2]>(), 128);
    }
}

/// The size requirements describing how big to make the buffers needed to create an acceleration structure.
#[derive(Debug, Copy, Clone)]
pub struct SizeRequirements {
    /// The required size for the acceleration structure buffer.
    pub acceleration_structure_size: u64,
    /// The required size for the scratch buffer used in the build step if an incremental update was requested.
    pub update_scratch_size: u64,
    /// The required size for the scratch buffer used in the build step.
    pub build_scratch_size: u64,
}

/// Denotes how an acceleration structure should be copied.
#[derive(Debug, Copy, Clone)]
pub enum CopyMode {
    /// Creates a copy of the source acceleration structure to the destination. Both must have been created with the same parameters.
    Copy,

    /// Creates a more compact version of the source acceleration structure into the destination. The destination acceleration structure must be at least large enough, as queried by `query::Type::AccelerationStructureCompactedSize`.
    Compact,
    // TODO(as-serialization)
    // /// TODO docs
    // Serialize,
    // /// TODO docs
    // Deserialize,
}

/// TODO better docs, read notes from https://www.khronos.org/registry/vulkan/specs/1.2-extensions/html/vkspec.html#VkAccelerationStructureBuildRangeInfoKHR
/// TODO `build_acceleration_structures_indirect` depends on the layout of this struct
#[derive(Debug)]
#[repr(C)]
pub struct BuildRangeDesc {
    // The range of primitives in the corresponding geometry to use for this acceleration structure build.
    /// TODO docs
    pub primitive_count: u32,
    /// TODO docs
    pub primitive_offset: u32,
    /// The index of the first vertex to use, in the case of a triangles geometry.
    // TODO is this not just primitive.start?
    pub first_vertex: u32,
    /// The additional offset into the transform buffer, in the case of a triangles geometry.
    pub transform_offset: u32,
}
