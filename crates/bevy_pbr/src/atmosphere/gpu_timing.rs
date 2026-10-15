//! Debug: per-pass GPU timings via pass-level `timestamp_writes`.
//!
//! `ATMOSPHERE_GPU_TIMING=<file.csv>` appends `frame,pass,ms` lines for the atmosphere passes.
//! Uses only `Features::TIMESTAMP_QUERY`, which wgpu exposes on Metal through
//! `MTLCounterSamplingPoint::AtStageBoundary` (Apple Silicon). Bevy's `RenderDiagnosticsPlugin`
//! writes timestamps *inside* passes/encoders instead, which needs dispatch/blit-boundary
//! sampling that Apple GPUs lack, so it reports CPU spans only there.
//! Times assume a 1 ns timestamp period (wgpu reports 1.0 on Apple Silicon/AMD).

use bevy_render::{
    render_resource::{
        Buffer, BufferDescriptor, BufferUsages, CommandEncoder, ComputePassTimestampWrites,
        MapMode, QuerySet, QuerySetDescriptor, QueryType, RenderPassTimestampWrites,
    },
    renderer::RenderDevice,
};
use std::{
    io::Write,
    sync::{Mutex, OnceLock},
};

const MAX_PASSES: u32 = 16;

struct State {
    query_set: QuerySet,
    resolve: Buffer,
    names: Vec<&'static str>,
    frame: u32,
    pending: Vec<(Buffer, Vec<&'static str>, u32)>,
}

static PATH: OnceLock<Option<String>> = OnceLock::new();
static STATE: OnceLock<Mutex<State>> = OnceLock::new();
// QuerySet clones share the same underlying set; kept here so pass descriptors can borrow it.
static QUERY_SET: OnceLock<QuerySet> = OnceLock::new();

fn path() -> Option<&'static str> {
    PATH.get_or_init(|| std::env::var("ATMOSPHERE_GPU_TIMING").ok()).as_deref()
}

fn state(device: &RenderDevice) -> Option<&'static Mutex<State>> {
    path()?;
    if !device.features().contains(bevy_render::settings::WgpuFeatures::TIMESTAMP_QUERY) {
        return None;
    }
    Some(STATE.get_or_init(|| {
        let query_set = device.wgpu_device().create_query_set(&QuerySetDescriptor {
            label: Some("atmosphere_gpu_timing"),
            ty: QueryType::Timestamp,
            count: MAX_PASSES * 2,
        });
        let _ = QUERY_SET.set(query_set.clone());
        let resolve = device.create_buffer(&BufferDescriptor {
            label: Some("atmosphere_gpu_timing_resolve"),
            size: (MAX_PASSES * 2 * 8) as u64,
            usage: BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        Mutex::new(State { query_set, resolve, names: Vec::new(), frame: 0, pending: Vec::new() })
    }))
}

fn slot(device: &RenderDevice, name: &'static str) -> Option<(&'static QuerySet, u32)> {
    let mut s = state(device)?.lock().unwrap();
    let i = s.names.len() as u32;
    if i >= MAX_PASSES {
        return None;
    }
    s.names.push(name);
    Some((QUERY_SET.get()?, i * 2))
}

pub fn compute(device: &RenderDevice, name: &'static str) -> Option<ComputePassTimestampWrites<'static>> {
    slot(device, name).map(|(q, i)| ComputePassTimestampWrites {
        query_set: q,
        beginning_of_pass_write_index: Some(i),
        end_of_pass_write_index: Some(i + 1),
    })
}

pub fn render(device: &RenderDevice, name: &'static str) -> Option<RenderPassTimestampWrites<'static>> {
    slot(device, name).map(|(q, i)| RenderPassTimestampWrites {
        query_set: q,
        beginning_of_pass_write_index: Some(i),
        end_of_pass_write_index: Some(i + 1),
    })
}

/// Call once per frame after the last timed pass was recorded (in the last encoder).
pub fn end_frame(device: &RenderDevice, encoder: &mut CommandEncoder) {
    let Some(st) = state(device) else { return };
    let mut s = st.lock().unwrap();
    let file = path().unwrap();
    // Map readbacks recorded on earlier frames (they have been submitted by now).
    let frame = s.frame;
    s.pending.retain(|(buf, names, f)| {
        if *f >= frame {
            return true;
        }
        let (b, names, f, file) = (buf.clone(), names.clone(), *f, file.to_string());
        buf.slice(..).map_async(MapMode::Read, move |r| {
            if r.is_err() {
                return;
            }
            let data = b.slice(..).get_mapped_range().map(|v| v.to_vec()).unwrap_or_default();
            let ts: Vec<u64> = data.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect();
            let mut out = String::new();
            for (k, n) in names.iter().enumerate() {
                let (a, e) = (ts[2 * k], ts[2 * k + 1]);
                if e > a {
                    out += &format!("{f},{n},{:.4}\n", (e - a) as f64 * 1e-6);
                }
            }
            if let Ok(mut fh) = std::fs::OpenOptions::new().create(true).append(true).open(&file) {
                let _ = fh.write_all(out.as_bytes());
            }
        });
        false
    });
    let n = s.names.len() as u32;
    if n > 0 {
        let readback = device.create_buffer(&BufferDescriptor {
            label: Some("atmosphere_gpu_timing_readback"),
            size: (n * 2 * 8) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.resolve_query_set(&s.query_set, 0..n * 2, &s.resolve, 0);
        encoder.copy_buffer_to_buffer(&s.resolve, 0, &readback, 0, (n * 2 * 8) as u64);
        let names = std::mem::take(&mut s.names);
        s.pending.push((readback, names, frame));
    }
    s.frame += 1;
}
