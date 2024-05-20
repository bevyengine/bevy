use std::{
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
};

use bevy_ecs::{system::Resource, world::World};

use crate::io::AssetSourceBuilder;

/// Private resource to store the temporary directory used by `temp://`.
/// Kept private as it should only be removed on application exit.
#[derive(Resource)]
enum TempDirectory {
    /// Uses [`TempDir`](tempfile::TempDir)'s drop behaviour to delete the directory.
    /// Note that this is not _guaranteed_ to succeed, so it is possible to leak files from this
    /// option until the underlying OS cleans temporary directories. For secure files, consider using
    /// [`tempfile`](tempfile::tempfile) directly.
    Delete(tempfile::TempDir),
    /// Will not delete the temporary directory on exit, leaving cleanup the responsibility of
    /// the user or their system.
    Persist(PathBuf),
}

impl TempDirectory {
    fn path(&self) -> &Path {
        match self {
            TempDirectory::Delete(x) => x.path(),
            TempDirectory::Persist(x) => x.as_ref(),
        }
    }
}

pub(crate) fn get_temp_source(
    world: &mut World,
    temporary_file_path: Option<String>,
) -> std::io::Result<AssetSourceBuilder> {
    let temp_dir = match world.remove_resource::<TempDirectory>() {
        Some(resource) => resource,
        None => match temporary_file_path {
            Some(path) => TempDirectory::Persist(path.into()),
            None => TempDirectory::Delete(tempfile::TempDir::with_prefix("bevy")?),
        },
    };

    let path = temp_dir
        .path()
        .as_os_str()
        .try_into()
        .map_err(|error| Error::new(ErrorKind::InvalidData, error))?;

    let source = AssetSourceBuilder::platform_default(path, None);

    world.insert_resource(temp_dir);

    Ok(source)
}
