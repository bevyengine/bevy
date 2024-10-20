use core::ops::Deref;

use bevy_asset::LoadContext;

use crate::{DataUri, GltfError};

/// A byte buffer from a glTF.
///
/// Can come from either a binary stream from within the glTF itself
/// or from a external file.
pub struct GltfBuffer(Vec<u8>);

impl GltfBuffer {
    const VALID_MIME_TYPES: &[&str] = &["application/octet-stream", "application/gltf-buffer"];

    /// Loads the raw glTF buffer data for a specific glTF file.
    pub async fn load_buffers(
        gltf: &gltf::Gltf,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Vec<GltfBuffer>, GltfError> {
        let mut buffer_data = Vec::new();
        for buffer in gltf.buffers() {
            match buffer.source() {
                gltf::buffer::Source::Uri(uri) => {
                    let uri = percent_encoding::percent_decode_str(uri)
                        .decode_utf8()
                        .unwrap();
                    let uri = uri.as_ref();
                    let buffer_bytes = match DataUri::parse(uri) {
                        Ok(data_uri) if Self::VALID_MIME_TYPES.contains(&data_uri.mime_type) => {
                            data_uri.decode()?
                        }
                        Ok(_) => return Err(GltfError::BufferFormatUnsupported),
                        Err(()) => {
                            // TODO: Remove this and add dep
                            let buffer_path = load_context.path().parent().unwrap().join(uri);
                            load_context.read_asset_bytes(buffer_path).await?
                        }
                    };
                    buffer_data.push(GltfBuffer(buffer_bytes));
                }
                gltf::buffer::Source::Bin => {
                    if let Some(blob) = gltf.blob.as_deref() {
                        buffer_data.push(GltfBuffer(blob.into()));
                    } else {
                        return Err(GltfError::MissingBlob);
                    }
                }
            }
        }

        Ok(buffer_data)
    }
}

impl Deref for GltfBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
