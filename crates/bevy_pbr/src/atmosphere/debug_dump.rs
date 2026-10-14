//! Debug: GPU texture readback for cloud validation.
//!
//! `CLOUD_DUMP_DIR=<dir>` + `CLOUD_DUMP_FRAMES=4,300` copies the listed textures to buffers on
//! those frames and writes raw RGBA16F/R32F files `<dir>/<name>_f<frame>_<w>x<h>x<d>.<fmt>` (rows
//! padded to 256 bytes; the padded width is in the file name).

use bevy_math::UVec3;
use bevy_render::{
    render_resource::{
        Buffer, BufferDescriptor, BufferUsages, CommandEncoder, MapMode, Origin3d,
        TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo, Texture, TextureAspect,
    },
    renderer::RenderDevice,
};
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Mutex,
};

static FRAME: AtomicU32 = AtomicU32::new(0);
static PENDING: Mutex<Vec<(Buffer, String, u32)>> = Mutex::new(Vec::new());

/// Returns the frame index if a dump is requested this frame; maps last frame's copies.
pub fn begin_frame() -> Option<u32> {
    let dir = std::env::var("CLOUD_DUMP_DIR").ok()?;
    let frame = FRAME.fetch_add(1, Ordering::Relaxed);
    let mut pending = PENDING.lock().unwrap();
    pending.retain(|(buffer, file, f)| {
        if *f >= frame {
            return true;
        }
        let b = buffer.clone();
        let file = format!("{dir}/{file}");
        buffer.slice(..).map_async(MapMode::Read, move |r| {
            if r.is_ok() {
                let data = b.slice(..).get_mapped_range().map(|v| v.to_vec()).unwrap_or_default();
                let _ = std::fs::write(&file, data);
                println!("cloud dump: {file}");
            }
        });
        false
    });
    let frames = std::env::var("CLOUD_DUMP_FRAMES").unwrap_or_else(|_| "4,300".into());
    frames
        .split(',')
        .filter_map(|v| v.trim().parse::<u32>().ok())
        .any(|f| f == frame)
        .then_some(frame)
}

/// Copies mip `mip` of `texture` (size already at that mip) into a readback buffer.
pub fn dump(
    device: &RenderDevice,
    encoder: &mut CommandEncoder,
    frame: u32,
    name: &str,
    texture: &Texture,
    mip: u32,
    size: UVec3,
    bytes_per_texel: u32,
    ext: &str,
) {
    let row = (size.x * bytes_per_texel).div_ceil(256) * 256;
    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some("cloud_debug_dump"),
        size: (row * size.y * size.z) as u64,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture,
            mip_level: mip,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(size.y),
            },
        },
        bevy_render::render_resource::Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: size.z,
        },
    );
    let file = format!(
        "{name}_f{frame:03}_{}x{}x{}.{ext}",
        row / bytes_per_texel,
        size.y,
        size.z
    );
    PENDING.lock().unwrap().push((buffer, file, frame));
}
