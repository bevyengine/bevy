//! `KHR_meshopt_compression` and `EXT_meshopt_compression` decoding.
//!
//! The extensions store a buffer view's data meshopt-encoded in another
//! buffer and point the view at a fallback buffer that files usually leave
//! empty. [`decode_buffer_views`] decodes into the fallback buffer, so
//! accessors read it like uncompressed data. The loader calls it after
//! loading buffers; code that reads a glTF with the [`gltf`] crate directly
//! can call it too.
//!
//! KHR adds the v1 vertex codec and the `COLOR` filter. The decoder reads
//! both vertex codec versions under either extension.

use core::ffi::{c_int, c_void};
use serde::Deserialize;

use crate::loader::{GltfError, KHR_MESHOPT_COMPRESSION, MESHOPT_COMPRESSION_EXTENSIONS};

/// The meshopt compression block on a buffer view.
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
    Color,
}

/// The meshopt compression extension on a glTF object and its value.
fn meshopt_extension<'a>(
    extension_value: impl Fn(&str) -> Option<&'a serde_json::Value>,
) -> Option<(&'static str, &'a serde_json::Value)> {
    MESHOPT_COMPRESSION_EXTENSIONS
        .into_iter()
        .find_map(|name| Some((name, extension_value(name)?)))
}

/// Whether `buffer` is a fallback buffer, which only compressed buffer views
/// use and [`decode_buffer_views`] fills in.
pub fn is_fallback_buffer(buffer: &gltf::Buffer) -> bool {
    meshopt_extension(|name| buffer.extension_value(name))
        .is_some_and(|(_, value)| value["fallback"] == true)
}

/// Decodes every buffer view that uses `KHR_meshopt_compression` or
/// `EXT_meshopt_compression` into its fallback buffer.
///
/// `buffer_data` holds one entry per glTF buffer, with fallback buffers
/// allocated to their `byteLength`. Malformed extension data returns
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
        let Some((extension, value)) = meshopt_extension(|name| view.extension_value(name)) else {
            continue;
        };
        decode_view(extension, value, &view, buffer_data).map_err(|message| {
            GltfError::InvalidMeshoptCompression {
                buffer_view: view.index(),
                message,
            }
        })?;
    }
    Ok(())
}

fn decode_view(
    extension: &str,
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

    let src = src
        .get(ext.byte_offset..)
        .and_then(|src| src.get(..ext.byte_length))
        .ok_or_else(|| {
            format!(
                "compressed range of {} bytes at {} exceeds buffer {} of {} bytes",
                ext.byte_length,
                ext.byte_offset,
                ext.buffer,
                src.len()
            )
        })?;

    let stride = ext.byte_stride;
    if ext.count.checked_mul(stride) != Some(view.length()) {
        return Err(format!(
            "count {} * byteStride {stride} does not match the buffer view's {} bytes",
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
    let dst_len = dst.len();
    let dst = dst
        .get_mut(view.offset()..)
        .and_then(|dst| dst.get_mut(..view.length()))
        .ok_or_else(|| {
            format!(
                "buffer view range of {} bytes at {} exceeds buffer {dst_buffer} of {dst_len} bytes",
                view.length(),
                view.offset()
            )
        })?;

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
        Filter::Color if extension != KHR_MESHOPT_COMPRESSION => {
            return Err(format!("COLOR filter requires {KHR_MESHOPT_COMPRESSION}"));
        }
        Filter::Color if stride != 4 && stride != 8 => {
            return Err(format!("COLOR byteStride {stride} is not 4 or 8"));
        }
        // EXPONENTIAL only needs the multiple-of-4 stride ATTRIBUTES already has.
        _ => {}
    }

    // The index decoders and filters store whole components, which glTF
    // aligns to their size.
    let align = match (ext.mode, ext.filter) {
        (Mode::Triangles | Mode::Indices, _) => stride,
        (_, Filter::None) => 1,
        (_, Filter::Octahedral | Filter::Color) => stride / 4,
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
    // SAFETY: `dst` is `count * stride` bytes. The
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
        Filter::Color => ffi::meshopt_decodeFilterColor,
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
pub(crate) mod fixture {
    use core::ffi::c_int;

    use super::KHR_MESHOPT_COMPRESSION;

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

    /// Triangles rotated to start at their smallest index. The index codec may
    /// rotate a triangle but keeps triangle order and winding.
    pub fn triangles(indices: &[u32]) -> Vec<[u32; 3]> {
        indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| {
                let start = (0..3).min_by_key(|&i| t[i]).unwrap();
                [t[start], t[(start + 1) % 3], t[(start + 2) % 3]]
            })
            .collect()
    }

    pub fn encode_vertices<T>(extension: &str, vertices: &[T]) -> Vec<u8> {
        let version = c_int::from(extension == KHR_MESHOPT_COMPRESSION);
        let size = size_of::<T>();
        #[expect(
            unsafe_code,
            reason = "meshopt only exposes the versioned encoder over FFI"
        )]
        // SAFETY: `encoded` holds the encoder's bound for `vertices`, which
        // holds `len * size` bytes.
        let encoded = unsafe {
            let mut encoded =
                vec![0; meshopt::ffi::meshopt_encodeVertexBufferBound(vertices.len(), size)];
            let len = meshopt::ffi::meshopt_encodeVertexBufferLevel(
                encoded.as_mut_ptr(),
                encoded.len(),
                vertices.as_ptr().cast(),
                vertices.len(),
                size,
                2,
                version,
            );
            encoded.truncate(len);
            encoded
        };
        assert!(!encoded.is_empty());
        encoded
    }

    /// A one-primitive document with positions in `bufferViews[0]` and u16
    /// indices in `bufferViews[1]`, both in URI-less fallback buffer 1, and
    /// the compressed data for buffer 0 (`mesh.bin`). The compressed data has
    /// a gap before the positions so their `byteOffset` is non-zero.
    pub fn fixture(
        extension: &str,
        positions: &[[f32; 3]],
        indices: &[u32],
    ) -> (serde_json::Value, Vec<u8>) {
        let encoded_indices = meshopt::encode_index_buffer(indices, positions.len()).unwrap();
        let encoded_positions = encode_vertices(extension, positions);

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
            "extensionsUsed": [extension],
            "extensionsRequired": [extension],
            "buffers": [
                { "byteLength": compressed.len(), "uri": "mesh.bin" },
                {
                    "byteLength": fallback_len,
                    "extensions": { extension: { "fallback": true } }
                }
            ],
            "bufferViews": [
                {
                    "buffer": 1, "byteOffset": 0, "byteLength": positions_len, "byteStride": 12,
                    "extensions": { extension: {
                        "buffer": 0, "byteOffset": positions_offset,
                        "byteLength": encoded_positions.len(),
                        "byteStride": 12, "count": positions.len(), "mode": "ATTRIBUTES"
                    } }
                },
                {
                    "buffer": 1, "byteOffset": positions_len, "byteLength": indices_len,
                    "extensions": { extension: {
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
        (json, compressed)
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::*;
    use super::*;
    use crate::loader::EXT_MESHOPT_COMPRESSION;

    /// The document and buffers for `json` with `compressed` in buffer 0 and
    /// zeroed fallback buffer 1.
    fn parse(json: serde_json::Value, compressed: Vec<u8>) -> (gltf::Document, Vec<Vec<u8>>) {
        let fallback_len = json["buffers"][1]["byteLength"].as_u64().unwrap() as usize;
        let root: gltf::json::Root = serde_json::from_value(json).unwrap();
        let document = gltf::Document::from_json_without_validation(root);
        (document, vec![compressed, vec![0u8; fallback_len]])
    }

    /// The fixture with `patch` applied to its JSON, and its buffers.
    fn document(
        extension: &str,
        positions: &[[f32; 3]],
        indices: &[u32],
        patch: impl FnOnce(&mut serde_json::Value),
    ) -> (gltf::Document, Vec<Vec<u8>>) {
        let (mut json, compressed) = fixture(extension, positions, indices);
        patch(&mut json);
        parse(json, compressed)
    }

    type FilterEncoder = unsafe extern "C" fn(*mut c_void, usize, usize, c_int, *const f32);

    /// `input` encoded by a meshopt filter encoder with `bits` per component.
    /// `STRIDE` must be the stride the encoder requires for `bits`.
    fn encode_filter<const STRIDE: usize>(
        encoder: FilterEncoder,
        bits: c_int,
        input: &[[f32; 4]],
    ) -> Vec<[u8; STRIDE]> {
        let mut filtered = vec![[0u8; STRIDE]; input.len()];
        #[expect(
            unsafe_code,
            reason = "meshopt only exposes the filter encoders over FFI"
        )]
        // SAFETY: `filtered` holds `STRIDE` bytes per vector and `input` four
        // floats per vector.
        unsafe {
            encoder(
                filtered.as_mut_ptr().cast(),
                input.len(),
                STRIDE,
                bits,
                input.as_ptr().cast(),
            );
        }
        filtered
    }

    /// Compresses `filtered` into one `ATTRIBUTES` view with `filter` and
    /// returns the decoded vertices.
    fn decode_filtered<const STRIDE: usize>(
        extension: &str,
        filter: &str,
        filtered: &[[u8; STRIDE]],
    ) -> Vec<[u8; STRIDE]> {
        let compressed = encode_vertices(extension, filtered);
        let fallback_len = filtered.len() * STRIDE;

        let json = serde_json::json!({
            "asset": { "version": "2.0" },
            "buffers": [
                { "byteLength": compressed.len(), "uri": "vertices.bin" },
                {
                    "byteLength": fallback_len,
                    "extensions": { extension: { "fallback": true } }
                }
            ],
            "bufferViews": [{
                "buffer": 1, "byteLength": fallback_len, "byteStride": STRIDE,
                "extensions": { extension: {
                    "buffer": 0, "byteLength": compressed.len(), "byteStride": STRIDE,
                    "count": filtered.len(), "mode": "ATTRIBUTES", "filter": filter
                } }
            }]
        });
        let (document, mut buffers) = parse(json, compressed);

        decode_buffer_views(&document, &mut buffers).unwrap();
        buffers[1].as_chunks::<STRIDE>().0.to_vec()
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
        let filtered = encode_filter::<8>(meshopt::ffi::meshopt_encodeFilterOct, 16, &normals);

        let decoded = decode_filtered(EXT_MESHOPT_COMPRESSION, "OCTAHEDRAL", &filtered);

        for (normal, decoded) in normals.iter().zip(&decoded) {
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
    fn round_trips_color_filter() {
        let colors: Vec<[f32; 4]> = (0..64)
            .map(|i| {
                let channel = |n: usize| ((i * n) % 17) as f32 / 16.0;
                [channel(3), channel(5), channel(7), channel(11)]
            })
            .collect();
        let filtered = encode_filter::<4>(meshopt::ffi::meshopt_encodeFilterColor, 8, &colors);

        let decoded = decode_filtered(KHR_MESHOPT_COMPRESSION, "COLOR", &filtered);

        for (color, decoded) in colors.iter().zip(&decoded) {
            for channel in 0..4 {
                let value = f32::from(decoded[channel]) / 255.0;
                assert!(
                    (value - color[channel]).abs() < 2.0 / 255.0,
                    "{color:?} decoded to {value} on channel {channel}"
                );
            }
        }
    }

    #[test]
    fn rejects_color_filter_without_khr() {
        let message = view_rejection(0, |json| {
            json["bufferViews"][0]["extensions"][EXT_MESHOPT_COMPRESSION]["filter"] =
                "COLOR".into();
        });
        assert!(
            message.contains("requires KHR_meshopt_compression"),
            "{message}"
        );
    }

    #[test]
    fn round_trips_positions_and_indices() {
        let positions = positions(97);
        let indices = indices(3 * 61, positions.len());
        for extension in MESHOPT_COMPRESSION_EXTENSIONS {
            let (document, mut buffers) = document(extension, &positions, &indices, |_| {});

            decode_buffer_views(&document, &mut buffers).unwrap();

            let expected_positions: Vec<u8> = positions
                .iter()
                .flatten()
                .flat_map(|f| f.to_le_bytes())
                .collect();
            assert_eq!(
                &buffers[1][..expected_positions.len()],
                &expected_positions[..]
            );
            let decoded_indices: Vec<u32> = buffers[1][expected_positions.len()..]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&b| u32::from(u16::from_le_bytes(b)))
                .collect();
            assert_eq!(triangles(&decoded_indices), triangles(&indices));
        }
    }

    /// The error message for `buffer_view` of the 16-vertex, 24-index EXT
    /// fixture after `patch`.
    fn view_rejection(buffer_view: usize, patch: impl FnOnce(&mut serde_json::Value)) -> String {
        let positions = positions(16);
        let indices = indices(3 * 8, positions.len());
        let (document, mut buffers) =
            document(EXT_MESHOPT_COMPRESSION, &positions, &indices, patch);

        match decode_buffer_views(&document, &mut buffers).unwrap_err() {
            GltfError::InvalidMeshoptCompression {
                buffer_view: view,
                message,
            } if view == buffer_view => message,
            err => panic!("unexpected error {err:?}"),
        }
    }

    #[test]
    fn rejects_compressed_range_past_buffer_end() {
        let message = view_rejection(1, |json| {
            json["bufferViews"][1]["extensions"][EXT_MESHOPT_COMPRESSION]["byteLength"] =
                (usize::MAX / 2).into();
        });
        assert!(message.contains("exceeds buffer 0"), "{message}");
    }

    #[test]
    fn rejects_truncated_compressed_data() {
        let message = view_rejection(1, |json| {
            json["bufferViews"][1]["extensions"][EXT_MESHOPT_COMPRESSION]["byteLength"] = 1.into();
        });
        assert!(message.contains("decoder returned"), "{message}");
    }

    #[test]
    fn rejects_count_not_matching_view_length() {
        let message = view_rejection(1, |json| {
            json["bufferViews"][1]["extensions"][EXT_MESHOPT_COMPRESSION]["count"] = 21.into();
        });
        assert!(message.contains("count 21 * byteStride 2"), "{message}");
    }

    #[test]
    fn rejects_triangle_count_not_multiple_of_three() {
        // Shrink the view alongside the count so only the triangle rule trips.
        let message = view_rejection(1, |json| {
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
        let message = view_rejection(1, |json| {
            json["bufferViews"][1]["byteLength"] = (15 * 3).into();
            let ext = &mut json["bufferViews"][1]["extensions"][EXT_MESHOPT_COMPRESSION];
            ext["byteStride"] = 3.into();
            ext["count"] = 15.into();
        });
        assert!(message.contains("byteStride 3 is not 2 or 4"), "{message}");
    }

    #[test]
    fn rejects_filter_outside_attributes_mode() {
        let message = view_rejection(1, |json| {
            json["bufferViews"][1]["extensions"][EXT_MESHOPT_COMPRESSION]["filter"] =
                "OCTAHEDRAL".into();
        });
        assert!(message.contains("only valid in ATTRIBUTES"), "{message}");
    }

    #[test]
    fn rejects_view_stride_mismatch() {
        let message = view_rejection(1, |json| {
            json["bufferViews"][1]["byteStride"] = 4.into();
        });
        assert!(message.contains("does not match byteStride 2"), "{message}");
    }

    #[test]
    fn rejects_misaligned_index_view() {
        let message = view_rejection(1, |json| {
            json["bufferViews"][1]["byteOffset"] = 191.into();
        });
        assert!(message.contains("not aligned to 2 bytes"), "{message}");
    }
}
