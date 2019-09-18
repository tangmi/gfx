// Copyright 2016 The Gfx-rs Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{cmp, mem, ptr};

use winapi::um::d3d11;
use winapi::shared::minwindef::UINT;
use winapi::shared::winerror::SUCCEEDED;

use core::{self, texture as tex};
use command;
use {Buffer, Texture};


fn copy_buffer(context: *mut d3d11::ID3D11DeviceContext,
               src: &Buffer, dst: &Buffer,
               src_offset: UINT, dst_offset: UINT,
               size: UINT) {
    let src_resource = src.as_resource();
    let dst_resource = dst.as_resource();
    let src_box = d3d11::D3D11_BOX {
        left: src_offset,
        right: src_offset + size,
        top: 0,
        bottom: 1,
        front: 0,
        back: 1,
    };
    unsafe {
        (*context).CopySubresourceRegion(dst_resource, 0, dst_offset, 0, 0,
                                         src_resource, 0, &src_box)
    };
}

fn copy_texture(context: *mut d3d11::ID3D11DeviceContext,
                src: &tex::TextureCopyRegion<Texture>,
                dst: &tex::TextureCopyRegion<Texture>) {
    assert_eq!((src.info.width, src.info.height, src.info.depth),
               (dst.info.width, dst.info.height, dst.info.depth));

    let (src_slice, src_front) = match src.kind.get_num_slices() {
        Some(_) => { assert!(src.info.depth <= 1); (src.info.zoffset, 0) },
        None => (0, src.info.zoffset),
    };
    let (dst_slice, dst_front) = match dst.kind.get_num_slices() {
        Some(_) => { assert!(dst.info.depth <= 1); (dst.info.zoffset, 0) },
        None => (0, dst.info.zoffset),
    };

    let src_box = d3d11::D3D11_BOX {
        left: src.info.xoffset as _,
        right: (src.info.xoffset + src.info.width) as _,
        top: src.info.yoffset as _,
        bottom: (src.info.yoffset + src.info.height) as _,
        front: src_front as _,
        back: (src_front + cmp::max(1, src.info.depth)) as _,
    };

    unsafe {
        let src_sub = d3d11::D3D11CalcSubresource(src.info.mipmap as _,
                                                   src.kind.get_num_levels() as _,
                                                   src_slice as _);
        let dst_sub = d3d11::D3D11CalcSubresource(dst.info.mipmap as _,
                                                   dst.kind.get_num_levels() as _,
                                                   dst_slice as _);
        (*context).CopySubresourceRegion(dst.texture.as_resource(), dst_sub,
                                         dst.info.xoffset as _, dst.info.yoffset as _, dst_front as _,
                                         src.texture.as_resource(), src_sub, &src_box)
    };

}

/// 4 copies, 1 texture allocation, 1 buffer alloction
fn copy_texture_to_buffer(context: *mut d3d11::ID3D11DeviceContext,
                src: &tex::TextureCopyRegion<Texture>,
                dst: &Buffer,
                dst_offset: UINT) {
    use crate::com_ptr::ComPtr;
    use crate::try_log;
    use winapi::shared::winerror;

    let (src_slice, src_front) = match src.kind.get_num_slices() {
        Some(_) => { assert!(src.info.depth <= 1); (src.info.zoffset, 0) },
        None => (0, src.info.zoffset),
    };

    // TODO only supports full copies?
    let _src_box = d3d11::D3D11_BOX {
        left: src.info.xoffset as _,
        right: (src.info.xoffset + src.info.width) as _,
        top: src.info.yoffset as _,
        bottom: (src.info.yoffset + src.info.height) as _,
        front: src_front as _,
        back: (src_front + cmp::max(1, src.info.depth)) as _,
    };

    let _src_sub = d3d11::D3D11CalcSubresource(src.info.mipmap as _,
                                              src.kind.get_num_levels() as _,
                                              src_slice as _);
    
    unsafe {
        let device = ComPtr::create_with(|out_ptr| {
            (*context).GetDevice(out_ptr);
            winerror::S_OK
        })
        // Note: GetDevice does not fail
        .unwrap();

        let immediate_context = ComPtr::create_with(|out_ptr| {
            device.GetImmediateContext(out_ptr);
            winerror::S_OK
        })
        // Note: GetImmediateContext does not fail
        .unwrap();

        // Copy src texture to a new staging texture
        let mut staging_texture = try_log!(
            "create staging texture",
            ComPtr::create_with(|out_ptr| {
                device.CreateTexture2D(
                    &d3d11::D3D11_TEXTURE2D_DESC {
                        Width: u32::from(src.info.width),
                        Height: u32::from(src.info.height),
                        MipLevels: 1,
                        ArraySize: 1,
                        Format: crate::data::map_format(src.info.format, false).expect("valid format combination"),                
                        SampleDesc: winapi::shared::dxgitype::DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        Usage: d3d11::D3D11_USAGE_STAGING,
                        BindFlags: 0,
                        CPUAccessFlags: d3d11::D3D11_CPU_ACCESS_READ,
                        MiscFlags: 0,
                    },
                    std::ptr::null(),
                    out_ptr,
                )
            })
        );

        // Copying src to staging
        immediate_context.CopySubresourceRegion(
            staging_texture.as_ptr() as *mut winapi::um::d3d11::ID3D11Resource,
            0,
            0, 0, 0,
            src.texture.as_resource(),
            0,
            std::ptr::null()
            // &src_box,
        );

        // Read staging texture to CPU
        // Must unmap
        let mut mapped_subresource: d3d11::D3D11_MAPPED_SUBRESOURCE = std::mem::zeroed();
        let hresult = immediate_context.Map(
            staging_texture.as_ptr() as *mut winapi::um::d3d11::ID3D11Resource,
            0,
            d3d11::D3D11_MAP_READ,
            0,
            &mut mapped_subresource,
        );
        if !SUCCEEDED(hresult) {
            error!("Failed to map staging texture, error {:x}", hresult);
            return;
        }

        let bytes_per_pixel = (src.info.format.0.get_total_bits() / 8) as usize;
        let width = src.info.width as usize;
        let height = src.info.height as usize;
        let depth = cmp::max(1, src.info.depth as usize);

        let dst_depth_pitch = width * height * bytes_per_pixel;
        let dst_row_pitch = width * bytes_per_pixel;

        let buffer_len = depth * dst_depth_pitch;
        let mut data = Vec::with_capacity(buffer_len);
        data.resize_with(buffer_len, Default::default);

        let src = mapped_subresource.pData as *const u8;
        assert!(!src.is_null());

        // Copying mapped data to CPU
        for slice in 0..depth {
            let slice_offset_src = slice * mapped_subresource.DepthPitch as usize;
            let slice_offset_dst = slice * dst_depth_pitch;

            for row in 0..height {
                let row_offset_src = slice_offset_src + row * mapped_subresource.RowPitch as usize;
                let row_offset_dst = slice_offset_dst + row * dst_row_pitch;

                for col in 0..width {
                    let pixel_offset_src = row_offset_src + col * bytes_per_pixel;
                    let pixel_offset_dst = row_offset_dst + col * bytes_per_pixel;

                    for byte in 0..bytes_per_pixel {
                        data[pixel_offset_dst + byte] = src.offset((pixel_offset_src + byte) as isize).read_unaligned();
                    }
                }
            }
        }

        immediate_context.Unmap(staging_texture.as_ptr() as *mut winapi::um::d3d11::ID3D11Resource, 0);

        // Copying CPU data to staging buffer
        let mut staging_buffer = try_log!(
            "create staging buffer",
            ComPtr::create_with(|out_ptr| {
                device.CreateBuffer(
                    &d3d11::D3D11_BUFFER_DESC {
                        ByteWidth: buffer_len as _,
                        Usage: d3d11::D3D11_USAGE_STAGING,
                        BindFlags: 0,
                        CPUAccessFlags: d3d11::D3D11_CPU_ACCESS_WRITE,
                        MiscFlags: 0,
                        StructureByteStride: 1,
                    },
                    &d3d11::D3D11_SUBRESOURCE_DATA {
                        pSysMem: data.as_ptr() as *const _,
                        SysMemPitch: buffer_len as _,
                        SysMemSlicePitch: buffer_len as _,
                    },
                    out_ptr,
                )
            })
        );

        // Copying staging buffer to dst buffer
        let src_box = d3d11::D3D11_BOX {
            left: 0,
            right: buffer_len as _,
            top: 0,
            bottom: 1,
            front: 0,
            back: 1,
        };

        immediate_context.CopySubresourceRegion(
            dst.as_resource(),
            0,
            dst_offset, 0, 0,
            staging_buffer.as_ptr() as *mut winapi::um::d3d11::ID3D11Resource,
            0,
            &src_box)

        // update_buffer(immediate_context, dst, data.as_slice(), dst_offset as usize);
    }
}

pub fn update_buffer(context: *mut d3d11::ID3D11DeviceContext, buffer: &Buffer,
                     data: &[u8], offset_bytes: usize) {
    let dst_resource = (buffer.0).0 as *mut d3d11::ID3D11Resource;

    // DYNAMIC only
    let map_type = d3d11::D3D11_MAP_WRITE_DISCARD;
    let hr = unsafe {
        let mut sub = mem::zeroed();
        let hr = (*context).Map(dst_resource, 0, map_type, 0, &mut sub);
        let dst = (sub.pData as *mut u8).offset(offset_bytes as isize);
        ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
        (*context).Unmap(dst_resource, 0);
        hr
    };
    if !SUCCEEDED(hr) {
        error!("Buffer {:?} failed to map, error {:x}", buffer, hr);
    }
}

pub fn update_texture(context: *mut d3d11::ID3D11DeviceContext,
                      tex: &tex::TextureCopyRegion<Texture>,
                      data: &[u8]) {
    let subres = texture_subres(tex.cube_face, &tex.info);
    let dst_resource = tex.texture.as_resource();
    // DYNAMIC only; This only works if the whole texture is covered.
    assert_eq!(tex.info.xoffset + tex.info.yoffset + tex.info.zoffset, 0);
    let map_type = d3d11::D3D11_MAP_WRITE_DISCARD;
    let hr = unsafe {
        let mut sub = mem::zeroed();
        let hr = (*context).Map(dst_resource, subres, map_type, 0, &mut sub);
        let dst = sub.pData as *mut u8;
        ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
        (*context).Unmap(dst_resource, 0);
        hr
    };
    if !SUCCEEDED(hr) {
        error!("Texture {:?} failed to map, error {:x}", tex.texture, hr);
    }
}

fn texture_subres(face: Option<tex::CubeFace>, image: &tex::RawImageInfo) -> UINT {
    use core::texture::CubeFace::*;

    let array_slice = match face {
        Some(PosX) => 0,
        Some(NegX) => 1,
        Some(PosY) => 2,
        Some(NegY) => 3,
        Some(PosZ) => 4,
        Some(NegZ) => 5,
        None => 0,
    };
    let num_mipmap_levels = 1; //TODO
    array_slice * num_mipmap_levels + (image.mipmap as UINT)
}

pub fn process(ctx: *mut d3d11::ID3D11DeviceContext, command: &command::Command, data_buf: &command::DataBuffer) {
    use core::shade::Stage;
    use command::Command::*;

    let max_cb  = core::MAX_CONSTANT_BUFFERS as UINT;
    let max_srv = core::MAX_RESOURCE_VIEWS   as UINT;
    let max_sm  = core::MAX_SAMPLERS         as UINT;
    //debug!("Processing {:?}", command);
    match *command {
        BindProgram(ref prog) => unsafe {
            (*ctx).VSSetShader(prog.vs, ptr::null_mut(), 0);
            (*ctx).HSSetShader(prog.hs, ptr::null_mut(), 0);
            (*ctx).DSSetShader(prog.ds, ptr::null_mut(), 0);
            (*ctx).GSSetShader(prog.gs, ptr::null_mut(), 0);
            (*ctx).PSSetShader(prog.ps, ptr::null_mut(), 0);
        },
        BindInputLayout(layout) => unsafe {
            (*ctx).IASetInputLayout(layout);
        },
        BindIndex(ref buf, format) => unsafe {
            (*ctx).IASetIndexBuffer((buf.0).0, format, 0);
        },
        BindVertexBuffers(ref buffers, ref strides, ref offsets) => unsafe {
            (*ctx).IASetVertexBuffers(0, core::MAX_VERTEX_ATTRIBUTES as _,
                &buffers[0].0, strides.as_ptr(), offsets.as_ptr());
        },
        BindConstantBuffers(stage, ref buffers) => match stage {
            Stage::Vertex => unsafe {
                (*ctx).VSSetConstantBuffers(0, max_cb, &buffers[0].0);
            },
            Stage::Hull => unsafe {
                (*ctx).HSSetConstantBuffers(0, max_cb, &buffers[0].0);
            },
            Stage::Domain => unsafe {
                (*ctx).DSSetConstantBuffers(0, max_cb, &buffers[0].0);
            },
            Stage::Geometry => unsafe {
                (*ctx).GSSetConstantBuffers(0, max_cb, &buffers[0].0);
            },
            Stage::Pixel => unsafe {
                (*ctx).PSSetConstantBuffers(0, max_cb, &buffers[0].0);
            },
        },
        BindShaderResources(stage, ref views) => match stage {
            Stage::Vertex => unsafe {
                (*ctx).VSSetShaderResources(0, max_srv, &views[0].0);
            },
            Stage::Hull => unsafe {
                (*ctx).HSSetShaderResources(0, max_srv, &views[0].0);
            },
            Stage::Domain => unsafe {
                (*ctx).DSSetShaderResources(0, max_srv, &views[0].0);
            },
            Stage::Geometry => unsafe {
                (*ctx).GSSetShaderResources(0, max_srv, &views[0].0);
            },
            Stage::Pixel => unsafe {
                (*ctx).PSSetShaderResources(0, max_srv, &views[0].0);
            },
        },
        BindSamplers(stage, ref samplers) => match stage {
            Stage::Vertex => unsafe {
                (*ctx).VSSetSamplers(0, max_sm, &samplers[0].0);
            },
            Stage::Hull => unsafe {
                (*ctx).HSSetSamplers(0, max_sm, &samplers[0].0);
            },
            Stage::Domain => unsafe {
                (*ctx).DSSetSamplers(0, max_sm, &samplers[0].0);
            },
            Stage::Geometry => unsafe {
                (*ctx).GSSetSamplers(0, max_sm, &samplers[0].0);
            },
            Stage::Pixel => unsafe {
                (*ctx).PSSetSamplers(0, max_sm, &samplers[0].0);
            },
        },
        BindPixelTargets(ref colors, ds) => unsafe {
            (*ctx).OMSetRenderTargets(core::MAX_COLOR_TARGETS as _,
                &colors[0].0, ds.0);
        },
        SetPrimitive(topology) => unsafe {
            (*ctx).IASetPrimitiveTopology(topology);
        },
        SetViewport(ref viewport) => unsafe {
            (*ctx).RSSetViewports(1, viewport);
        },
        SetScissor(ref rect) => unsafe {
            (*ctx).RSSetScissorRects(1, rect);
        },
        SetRasterizer(rast) => unsafe {
            (*ctx).RSSetState(rast as *mut _);
        },
        SetDepthStencil(ds, value) => unsafe {
            (*ctx).OMSetDepthStencilState(ds as *mut _, value);
        },
        SetBlend(blend, ref value, mask) => unsafe {
            (*ctx).OMSetBlendState(blend as *mut _, value, mask);
        },
        CopyBuffer(ref src, ref dst, src_offset, dst_offset, size) => {
            copy_buffer(ctx, src, dst, src_offset, dst_offset, size);
        },
        CopyTexture(ref src, ref dst) => {
            copy_texture(ctx, src, dst);
        },
        CopyTextureToBuffer(ref src, ref dst, dst_offset) => {
            copy_texture_to_buffer(ctx, src, dst, dst_offset);
        },
        UpdateBuffer(ref buffer, pointer, offset) => {
            let data = data_buf.get(pointer);
            update_buffer(ctx, buffer, data, offset);
        },
        UpdateTexture(ref dst, pointer) => {
            let data = data_buf.get(pointer);
            update_texture(ctx, dst, data);
        },
        GenerateMips(ref srv) => unsafe {
            (*ctx).GenerateMips(srv.0);
        },
        ClearColor(target, ref data) => unsafe {
            (*ctx).ClearRenderTargetView(target.0, data);
        },
        ClearDepthStencil(target, flags, depth, stencil) => unsafe {
            (*ctx).ClearDepthStencilView(target.0, flags, depth, stencil);
        },
        Draw(nvert, svert) => unsafe {
            (*ctx).Draw(nvert, svert);
        },
        DrawInstanced(nvert, ninst, svert, sinst) => unsafe {
            (*ctx).DrawInstanced(nvert, ninst, svert, sinst);
        },
        DrawIndexed(nind, svert, base) => unsafe {
            (*ctx).DrawIndexed(nind, svert, base);
        },
        DrawIndexedInstanced(nind, ninst, sind, base, sinst) => unsafe {
            (*ctx).DrawIndexedInstanced(nind, ninst, sind, base, sinst);
        },
    }
}
