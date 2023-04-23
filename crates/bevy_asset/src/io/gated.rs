use crate::io::{AssetReader, AssetReaderError, PathStream, Reader};
use anyhow::Result;
use bevy_utils::{BoxedFuture, HashMap};
use crossbeam_channel::{Receiver, Sender};
use parking_lot::RwLock;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct GatedReader<R: AssetReader> {
    reader: R,
    gates: Arc<RwLock<HashMap<PathBuf, (Sender<()>, Receiver<()>)>>>,
}

impl<R: AssetReader + Clone> Clone for GatedReader<R> {
    fn clone(&self) -> Self {
        Self {
            reader: self.reader.clone(),
            gates: self.gates.clone(),
        }
    }
}

pub struct GateOpener {
    gates: Arc<RwLock<HashMap<PathBuf, (Sender<()>, Receiver<()>)>>>,
}

impl GateOpener {
    pub fn open<P: AsRef<Path>>(&self, path: P) {
        let mut gates = self.gates.write();
        let gates = gates
            .entry(path.as_ref().to_path_buf())
            .or_insert_with(|| crossbeam_channel::unbounded());
        gates.0.send(()).unwrap();
    }
}

impl<R: AssetReader> GatedReader<R> {
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
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        let receiver = {
            let mut gates = self.gates.write();
            let gates = gates
                .entry(path.to_path_buf())
                .or_insert_with(|| crossbeam_channel::unbounded());
            gates.1.clone()
        };
        Box::pin(async move {
            let _ = receiver.recv().unwrap();
            let result = self.reader.read(path).await?;
            Ok(result)
        })
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        self.reader.read_meta(path)
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        self.reader.read_directory(path)
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, std::result::Result<bool, AssetReaderError>> {
        self.reader.is_directory(path)
    }

    fn watch_for_changes(
        &self,
        event_sender: Sender<super::AssetSourceEvent>,
    ) -> Option<Box<dyn super::AssetWatcher>> {
        self.reader.watch_for_changes(event_sender)
    }
}
