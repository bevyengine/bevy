use crate::io::{AssetReader, AssetReaderError, PathStream, Reader};
use bevy_utils::HashMap;
use crossbeam_channel::{Receiver, Sender};
use parking_lot::RwLock;
use std::{path::Path, sync::Arc};

/// A "gated" reader that will prevent asset reads from returning until
/// a given path has been "opened" using [`GateOpener`].
///
/// This is built primarily for unit tests.
pub struct GatedReader<R: AssetReader> {
    reader: R,
    gates: Arc<RwLock<HashMap<Box<Path>, (Sender<()>, Receiver<()>)>>>,
}

impl<R: AssetReader + Clone> Clone for GatedReader<R> {
    fn clone(&self) -> Self {
        Self {
            reader: self.reader.clone(),
            gates: self.gates.clone(),
        }
    }
}

/// Opens path "gates" for a [`GatedReader`].
pub struct GateOpener {
    gates: Arc<RwLock<HashMap<Box<Path>, (Sender<()>, Receiver<()>)>>>,
}

impl GateOpener {
    /// Opens the `path` "gate", allowing a _single_ [`AssetReader`] operation to return for that path.
    /// If multiple operations are expected, call `open` the expected number of calls.
    pub fn open<P: AsRef<Path>>(&self, path: P) {
        let mut gates = self.gates.write();
        let gates = gates
            .entry_ref(path.as_ref())
            .or_insert_with(crossbeam_channel::unbounded);
        gates.0.send(()).unwrap();
    }
}

impl<R: AssetReader> GatedReader<R> {
    /// Creates a new [`GatedReader`], which wraps the given `reader`. Also returns a [`GateOpener`] which
    /// can be used to open "path gates" for this [`GatedReader`].
    pub fn new(reader: R) -> (Self, GateOpener) {
        let gates = Arc::new(RwLock::new(HashMap::new()));
        (
            Self {
                reader,
                gates: gates.clone(),
            },
            GateOpener { gates },
        )
    }
}

impl<R: AssetReader> AssetReader for GatedReader<R> {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let receiver = {
            let mut gates = self.gates.write();
            let gates = gates
                .entry_ref(path.as_ref())
                .or_insert_with(crossbeam_channel::unbounded);
            gates.1.clone()
        };
        receiver.recv().unwrap();
        let result = self.reader.read(path).await?;
        Ok(result)
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        self.reader.read_meta(path).await
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        self.reader.read_directory(path).await
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        self.reader.is_directory(path).await
    }
}
