//! `EXT_meshopt_compression` decoding.
//!
//! The extension stores a buffer view's data meshopt-encoded in another
//! buffer and points the view at a fallback buffer that files usually leave
//! empty. [`decode_buffer_views`] decodes into the fallback buffer, so
//! accessors read it like uncompressed data. The loader calls it after
//! loading buffers; code that reads a glTF with the [`gltf`] crate directly
//! can call it too.

use core::ffi::{c_int, c_void};
use serde::Deserialize;

use super::{GltfError, EXT_MESHOPT_COMPRESSION};

/// The `EXT_meshopt_compression` block on a buffer view.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ViewExtension {
    buffer: usize,
    #[serde(default)]
    byte_offset: usize,
    byte_length: usize,
    byte_stride: usize,
    count: usize,
    mode: Mode,
    #[serde(default)]
    filter: Filter,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Mode {
    Attributes,
    Triangles,
    Indices,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Filter {
    #[default]
    None,
    Octahedral,
    Quaternion,
    Exponential,
}

/// Decodes every buffer view that uses `EXT_meshopt_compression` into its
/// fallback buffer.
///
/// `buffer_data` holds one entry per glTF buffer, with URI-less fallback
/// buffers allocated to their `byteLength`. Malformed extension data returns
/// [`GltfError::InvalidMeshoptCompression`] before anything reaches the
/// decoder.
///
/// # Panics
///
/// If a buffer view's `buffer` index is out of range, which the `gltf`
/// crate's validation rejects.
pub fn decode_buffer_views(
    document: &gltf::Document,
    buffer_data: &mut [Vec<u8>],
) -> Result<(), GltfError> {
    for view in document.views() {
        let Some(value) = view.extension_value(EXT_MESHOPT_COMPRESSION) else {
            continue;
        };
        decode_view(value, &view, buffer_data).map_err(|message| {
            GltfError::InvalidMeshoptCompression {
                buffer_view: view.index(),
                message,
            }
        })?;
    }
    Ok(())
}

fn decode_view(
    value: &serde_json::Value,
    view: &gltf::buffer::View,
    buffer_data: &mut [Vec<u8>],
) -> Result<(), String> {
    let ext = ViewExtension::deserialize(value).map_err(|err| err.to_string())?;

    let dst_buffer = view.buffer().index();
    if ext.buffer == dst_buffer {
        return Err(format!(
            "compressed data and buffer view both live in buffer {dst_buffer}"
        ));
    }
    let [src, dst] = buffer_data
        .get_disjoint_mut([ext.buffer, dst_buffer])
        .map_err(|_| format!("compressed buffer {} does not exist", ext.buffer))?;

    let src_end = ext
        .byte_offset
        .checked_add(ext.byte_length)
        .ok_or("compressed range overflows")?;
    if src_end > src.len() {
        return Err(format!(
            "compressed range {}..{src_end} exceeds buffer {} of {} bytes",
            ext.byte_offset,
            ext.buffer,
            src.len()
        ));
    }

    let stride = ext.byte_stride;
    let decoded_len = ext
        .count
        .checked_mul(stride)
        .ok_or("count * byteStride overflows")?;
    if decoded_len != view.length() {
        return Err(format!(
            "count {} * byteStride {stride} = {decoded_len} bytes but the buffer view holds {}",
            ext.count,
            view.length()
        ));
    }
    if let Some(view_stride) = view.stride()
        && view_stride != stride
    {
        return Err(format!(
            "buffer view byteStride {view_stride} does not match byteStride {stride}"
        ));
    }
    let dst_end = view
        .offset()
        .checked_add(decoded_len)
        .ok_or("buffer view range overflows")?;
    if dst_end > dst.len() {
        return Err(format!(
            "buffer view range {}..{dst_end} exceeds buffer {dst_buffer} of {} bytes",
            view.offset(),
            dst.len()
        ));
    }

    match ext.mode {
        Mode::Attributes if stride == 0 || !stride.is_multiple_of(4) || stride > 256 => {
            return Err(format!(
                "ATTRIBUTES byteStride {stride} is not a multiple of 4 in 4..=256"
            ));
        }
        Mode::Triangles | Mode::Indices if stride != 2 && stride != 4 => {
            return Err(format!("{:?} byteStride {stride} is not 2 or 4", ext.mode));
        }
        // The triangle decoder only asserts this and otherwise writes past `dst`.
        Mode::Triangles if !ext.count.is_multiple_of(3) => {
            return Err(format!(
                "TRIANGLES count {} is not a multiple of 3",
                ext.count
            ));
        }
        _ => {}
    }
    match ext.filter {
        Filter::None => {}
        _ if ext.mode != Mode::Attributes => {
            return Err(format!(
                "filter {:?} is only valid in ATTRIBUTES mode",
                ext.filter
            ));
        }
        Filter::Octahedral if stride != 4 && stride != 8 => {
            return Err(format!("OCTAHEDRAL byteStride {stride} is not 4 or 8"));
        }
        Filter::Quaternion if stride != 8 => {
            return Err(format!("QUATERNION byteStride {stride} is not 8"));
        }
        // EXPONENTIAL only needs the multiple-of-4 stride ATTRIBUTES already has.
        _ => {}
    }

    let src = &src[ext.byte_offset..src_end];
    let dst = &mut dst[view.offset()..dst_end];

    // The index decoders and filters store whole components, which glTF
    // aligns to their size.
    let align = match (ext.mode, ext.filter) {
        (Mode::Triangles | Mode::Indices, _) => stride,
        (_, Filter::None) => 1,
        (_, Filter::Octahedral) => stride / 4,
        (_, Filter::Quaternion) => 2,
        (_, Filter::Exponential) => 4,
    };
    if !dst.as_ptr().addr().is_multiple_of(align) {
        return Err(format!(
            "buffer view byteOffset {} is not aligned to {align} bytes",
            view.offset()
        ));
    }

    use meshopt::ffi;
    type Decoder = unsafe extern "C" fn(*mut c_void, usize, usize, *const u8, usize) -> c_int;
    let decoder: Decoder = match ext.mode {
        Mode::Attributes => ffi::meshopt_decodeVertexBuffer,
        Mode::Triangles => ffi::meshopt_decodeIndexBuffer,
        Mode::Indices => ffi::meshopt_decodeIndexSequence,
    };
    // SAFETY: `dst` is `count * stride` bytes and `src` is in bounds. The
    // stride, TRIANGLES count and alignment meet the decoder's preconditions,
    // and the decoders are safe for untrusted `src`.
    #[expect(unsafe_code, reason = "meshopt only exposes these decoders over FFI")]
    let result = unsafe {
        decoder(
            dst.as_mut_ptr().cast(),
            ext.count,
            stride,
            src.as_ptr(),
            src.len(),
        )
    };
    if result != 0 {
        return Err(format!("meshopt decoder returned {result}"));
    }

    // Filters decode in place and cannot fail.
    type FilterDecoder = unsafe extern "C" fn(*mut c_void, usize, usize);
    let filter: FilterDecoder = match ext.filter {
        Filter::None => return Ok(()),
        Filter::Octahedral => ffi::meshopt_decodeFilterOct,
        Filter::Quaternion => ffi::meshopt_decodeFilterQuat,
        Filter::Exponential => ffi::meshopt_decodeFilterExp,
    };
    // SAFETY: `dst` is `count * stride` bytes, and the stride and alignment
    // meet the filter's preconditions.
    #[expect(unsafe_code, reason = "meshopt only exposes the filters over FFI")]
    unsafe {
        filter(dst.as_mut_ptr().cast(), ext.count, stride);
    }

    Ok(())
}

/// Meshopt-encoded geometry laid out like gltfpack output, shared with the
/// loader tests.
#[cfg(test)]
pub(super) mod fixture {
    use super::EXT_MESHOPT_COMPRESSION;

    /// Positions whose x coordinate is half the vertex index.
    pub fn positions(count: usize) -> Vec<[f32; 3]> {
        (0..count)
            .map(|i| [i as f32 * 0.5, (i % 7) as f32, -(i as f32)])
            .collect()
    }

    pub fn indices(count: usize, vertex_count: usize) -> Vec<u32> {
        (0..count)
            .map(|i| ((i * 7) % vertex_count) as u32)
            .collect()
    }

    pub fn position_bytes(positions: &[[f32; 3]]) -> Vec<u8> {
        positions
            .iter()
            .flatten()
            .flat_map(|f| f.to_le_bytes())
            .collect()
    }

    /// Triangles rotated to start at their smallest index. The index codec may
    /// rotate a triangle but keeps triangle order and winding.
    pub fn triangles(indices: &[u32]) -> Vec<[u32; 3]> {
        indices
            .chunks_exact(3)
            .map(|t| {
                let start = (0..3).min_by_key(|&i| t[i]).unwrap();
                [t[start], t[(start + 1) % 3], t[(start + 2) % 3]]
            })
            .collect()
    }

    /// A one-primitive document with positions in `bufferViews[0]` and u16
    /// indices in `bufferViews[1]`, both in URI-less fallback buffer 1.
    /// Buffer 0 (`mesh.bin`) holds `compressed`, with a gap before the
    /// positions so their `byteOffset` is non-zero.
    pub struct Fixture {
        pub json: serde_json::Value,
        pub compressed: Vec<u8>,
        pub fallback_len: usize,
    }

    pub fn fixture(positions: &[[f32; 3]], indices: &[u32]) -> Fixture {
        let encoded_indices = meshopt::encode_index_buffer(indices, positions.len()).unwrap();
        let encoded_positions = meshopt::encode_vertex_buffer(positions).unwrap();

        const PADDING: usize = 13;
        let mut compressed = encoded_indices.clone();
        compressed.extend(core::iter::repeat_n(0xAA, PADDING));
        let positions_offset = compressed.len();
        compressed.extend_from_slice(&encoded_positions);

        let positions_len = positions.len() * 12;
        let indices_len = indices.len() * 2;
        let fallback_len = positions_len + indices_len;
        // Validation requires POSITION min and max.
        let bound = |pick: fn(f32, f32) -> f32| -> [f32; 3] {
            core::array::from_fn(|axis| {
                positions
                    .iter()
                    .map(|p| p[axis])
                    .fold(positions[0][axis], pick)
            })
        };
        let (min, max) = (bound(f32::min), bound(f32::max));

        let json = serde_json::json!({
            "asset": { "version": "2.0" },
            "extensionsUsed": [EXT_MESHOPT_COMPRESSION],
            "extensionsRequired": [EXT_MESHOPT_COMPRESSION],
            "buffers": [
                { "byteLength": compressed.len(), "uri": "mesh.bin" },
                {
                    "byteLength": fallback_len,
                    "extensions": { EXT_MESHOPT_COMPRESSION: { "fallback": true } }
                }
            ],
            "bufferViews": [
                {
                    "buffer": 1, "byteOffset": 0, "byteLength": positions_len, "byteStride": 12,
                    "extensions": { EXT_MESHOPT_COMPRESSION: {
                        "buffer": 0, "byteOffset": positions_offset,
                        "byteLength": encoded_positions.len(),
                        "byteStride": 12, "count": positions.len(), "mode": "ATTRIBUTES"
                    } }
                },
                {
                    "buffer": 1, "byteOffset": positions_len, "byteLength": indices_len,
                    "extensions": { EXT_MESHOPT_COMPRESSION: {
                        "buffer": 0,
                        "byteLength": encoded_indices.len(),
                        "byteStride": 2, "count": indices.len(), "mode": "TRIANGLES"
                    } }
                }
            ],
            "accessors": [
                {
                    "bufferView": 0, "componentType": 5126, "count": positions.len(), "type": "VEC3",
                    "min": min, "max": max
                },
                { "bufferView": 1, "componentType": 5123, "count": indices.len(), "type": "SCALAR" }
            ],
            "meshes": [
                { "primitives": [{ "attributes": { "POSITION": 0 }, "indices": 1 }] }
            ]
        });
        Fixture {
            json,
            compressed,
            fallback_len,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::*;
    use super::*;

    /// The fixture with `patch` applied to its JSON, and its buffers.
    fn document(
        positions: &[[f32; 3]],
        indices: &[u32],
        patch: impl FnOnce(&mut serde_json::Value),
    ) -> (gltf::Document, Vec<Vec<u8>>) {
        let Fixture {
            mut json,
            compressed,
            fallback_len,
        } = fixture(positions, indices);
        patch(&mut json);
        let root: gltf::json::Root = serde_json::from_value(json).unwrap();
        let document = gltf::Document::from_json_without_validation(root);
        (document, vec![compressed, vec![0u8; fallback_len]])
    }

    #[test]
    fn round_trips_octahedral_filter() {
        let normals: Vec<[f32; 4]> = (0..64)
            .map(|i: i32| {
                let v = bevy_math::Vec3::new(
                    (i % 9 - 4) as f32,
                    (i * 5 % 11 - 5) as f32,
                    (i * 3 % 7 - 3) as f32,
                )
                .normalize_or(bevy_math::Vec3::Z);
                [v.x, v.y, v.z, 0.0]
            })
            .collect();
        const STRIDE: usize = 8;
        let mut filtered = vec![[0u8; STRIDE]; normals.len()];
        #[expect(
            unsafe_code,
            reason = "meshopt only exposes the filter encoders over FFI"
        )]
        // SAFETY: `filtered` holds `count * 8` bytes, the stride the 16-bit
        // encoding requires, and `normals` holds four floats per vector.
        unsafe {
            meshopt::ffi::meshopt_encodeFilterOct(
                filtered.as_mut_ptr().cast(),
                normals.len(),
                STRIDE,
                16,
                normals.as_ptr().cast(),
            );
        }
        let compressed = meshopt::encode_vertex_buffer(&filtered).unwrap();
        let fallback_len = normals.len() * STRIDE;

        let json = serde_json::json!({
            "asset": { "version": "2.0" },
            "buffers": [
                { "byteLength": compressed.len(), "uri": "normals.bin" },
                {
                    "byteLength": fallback_len,
                    "extensions": { EXT_MESHOPT_COMPRESSION: { "fallback": true } }
                }
            ],
            "bufferViews": [{
                "buffer": 1, "byteLength": fallback_len, "byteStride": STRIDE,
                "extensions": { EXT_MESHOPT_COMPRESSION: {
                    "buffer": 0, "byteLength": compressed.len(), "byteStride": STRIDE,
                    "count": normals.len(), "mode": "ATTRIBUTES", "filter": "OCTAHEDRAL"
                } }
            }]
        });
        let root: gltf::json::Root = serde_json::from_value(json).unwrap();
        let document = gltf::Document::from_json_without_validation(root);
        let mut buffers = vec![compressed, vec![0u8; fallback_len]];

        decode_buffer_views(&document, &mut buffers).unwrap();

        for (normal, decoded) in normals.iter().zip(buffers[1].chunks_exact(STRIDE)) {
            for axis in 0..3 {
                let value = i16::from_le_bytes([decoded[axis * 2], decoded[axis * 2 + 1]]);
                let value = f32::from(value) / f32::from(i16::MAX);
                assert!(
                    (value - normal[axis]).abs() < 1e-3,
                    "{normal:?} decoded to {value} on axis {axis}"
                );
            }
        }
    }

    #[test]
    fn round_trips_positions_and_indices() {
        let positions = positions(97);
        let indices = indices(3 * 61, positions.len());
        let (document, mut buffers) = document(&positions, &indices, |_| {});

        decode_buffer_views(&document, &mut buffers).unwrap();

        let expected_positions = position_bytes(&positions);
        assert_eq!(
            &buffers[1][..expected_positions.len()],
            &expected_positions[..]
        );
        let decoded_indices: Vec<u32> = buffers[1][expected_positions.len()..]
            .chunks_exact(2)
            .map(|b| u32::from(u16::from_le_bytes([b[0], b[1]])))
            .collect();
        assert_eq!(triangles(&decoded_indices), triangles(&indices));
    }

    /// The error message for the 24-index view after `patch`.
    fn index_view_rejection(patch: impl FnOnce(&mut serde_json::Value)) -> String {
        let positions = positions(16);
        let indices = indices(3 * 8, positions.len());
        let (document, mut buffers) = document(&positions, &indices, patch);

        match decode_buffer_views(&document, &mut buffers).unwrap_err() {
            GltfError::InvalidMeshoptCompression {
                buffer_view: 1,
                message,
            } => message,
            err => panic!("unexpected error {err:?}"),
        }
    }

    #[test]
    fn rejects_compressed_range_past_buffer_end() {
        let message = index_view_rejection(|json| {
            json["bufferViews"][1]["extensions"][EXT_MESHOPT_COMPRESSION]["byteLength"] =
                (usize::MAX / 2).into();
        });
        assert!(message.contains("exceeds buffer 0"), "{message}");
    }

    #[test]
    fn rejects_truncated_compressed_data() {
        let message = index_view_rejection(|json| {
            json["bufferViews"][1]["extensions"][EXT_MESHOPT_COMPRESSION]["byteLength"] = 1.into();
        });
        assert!(message.contains("decoder returned"), "{message}");
    }

    #[test]
    fn rejects_count_not_matching_view_length() {
        let message = index_view_rejection(|json| {
            json["bufferViews"][1]["extensions"][EXT_MESHOPT_COMPRESSION]["count"] = 21.into();
        });
        assert!(message.contains("count 21 * byteStride 2"), "{message}");
    }

    #[test]
    fn rejects_triangle_count_not_multiple_of_three() {
        // Shrink the view alongside the count so only the triangle rule trips.
        let message = index_view_rejection(|json| {
            json["bufferViews"][1]["byteLength"] = (22 * 2).into();
            json["bufferViews"][1]["extensions"][EXT_MESHOPT_COMPRESSION]["count"] = 22.into();
        });
        assert!(
            message.contains("count 22 is not a multiple of 3"),
            "{message}"
        );
    }

    #[test]
    fn rejects_index_stride_other_than_two_or_four() {
        // Keep count * byteStride inside the view's 48-byte fallback region.
        let message = index_view_rejection(|json| {
            json["bufferViews"][1]["byteLength"] = (15 * 3).into();
            let ext = &mut json["bufferViews"][1]["extensions"][EXT_MESHOPT_COMPRESSION];
            ext["byteStride"] = 3.into();
            ext["count"] = 15.into();
        });
        assert!(message.contains("byteStride 3 is not 2 or 4"), "{message}");
    }

    #[test]
    fn rejects_filter_outside_attributes_mode() {
        let message = index_view_rejection(|json| {
            json["bufferViews"][1]["extensions"][EXT_MESHOPT_COMPRESSION]["filter"] =
                "OCTAHEDRAL".into();
        });
        assert!(message.contains("only valid in ATTRIBUTES"), "{message}");
    }

    #[test]
    fn rejects_view_stride_mismatch() {
        let message = index_view_rejection(|json| {
            json["bufferViews"][1]["byteStride"] = 4.into();
        });
        assert!(message.contains("does not match byteStride 2"), "{message}");
    }

    #[test]
    fn rejects_misaligned_index_view() {
        let message = index_view_rejection(|json| {
            json["bufferViews"][1]["byteOffset"] = 191.into();
        });
        assert!(message.contains("not aligned to 2 bytes"), "{message}");
    }
}
