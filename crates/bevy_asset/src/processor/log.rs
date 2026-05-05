use crate::AssetPath;
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::Vec,
};
use async_fs::File;
use bevy_ecs::error::BevyError;
use bevy_platform::collections::HashSet;
use bevy_tasks::BoxedFuture;
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use std::path::PathBuf;
use thiserror::Error;
use tracing::error;

/// An in-memory representation of a single [`ProcessorTransactionLog`] entry.
#[derive(Debug)]
pub enum LogEntry {
    BeginProcessing(AssetPath<'static>),
    EndProcessing(AssetPath<'static>),
    UnrecoverableError,
}

/// A factory of [`ProcessorTransactionLog`] that handles the state before the log has been started.
///
/// This trait also assists in recovering from partial processing by fetching the previous state of
/// the transaction log.
pub trait ProcessorTransactionLogFactory: Send + Sync + 'static {
    /// Reads all entries in a previous transaction log if present.
    ///
    /// If there is no previous transaction log, this method should return an empty Vec of entries.
    fn read(&self) -> BoxedFuture<'_, Result<Vec<LogEntry>, BevyError>>;

    /// Creates a new transaction log to write to.
    ///
    /// This should remove any previous entries if they exist.
    fn create_new_log(
        &self,
    ) -> BoxedFuture<'_, Result<Box<dyn ProcessorTransactionLog>, BevyError>>;
}

/// A "write ahead" logger that helps ensure asset importing is transactional.
///
/// Prior to processing an asset, we write to the log to indicate it has started. After processing
/// an asset, we write to the log to indicate it has finished. On startup, the log can be read
/// through [`ProcessorTransactionLogFactory`] to determine if any transactions were incomplete.
pub trait ProcessorTransactionLog: Send + Sync + 'static {
    /// Logs the start of an asset being processed.
    ///
    /// If this is not followed at some point in the log by a closing
    /// [`ProcessorTransactionLog::end_processing`], in the next run of the processor the asset
    /// processing will be considered "incomplete" and it will be reprocessed.
    fn begin_processing<'a>(
        &'a mut self,
        asset: &'a AssetPath<'_>,
    ) -> BoxedFuture<'a, Result<(), BevyError>>;

    /// Logs the end of an asset being successfully processed. See
    /// [`ProcessorTransactionLog::begin_processing`].
    fn end_processing<'a>(
        &'a mut self,
        asset: &'a AssetPath<'_>,
    ) -> BoxedFuture<'a, Result<(), BevyError>>;

    /// Logs an unrecoverable error.
    ///
    /// On the next run of the processor, all assets will be regenerated. This should only be used
    /// as a last resort. Every call to this should be considered with scrutiny and ideally replaced
    /// with something more granular.
    fn unrecoverable(&mut self) -> BoxedFuture<'_, Result<(), BevyError>>;
}

/// Validate the previous state of the transaction log and determine any assets that need to be
/// reprocessed.
pub(crate) async fn validate_transaction_log(
    log_factory: &dyn ProcessorTransactionLogFactory,
) -> Result<(), ValidateLogError> {
    let mut transactions: HashSet<AssetPath<'static>> = Default::default();
    let mut errors: Vec<LogEntryError> = Vec::new();
    let entries = log_factory
        .read()
        .await
        .map_err(ValidateLogError::ReadLogError)?;
    for entry in entries {
        match entry {
            LogEntry::BeginProcessing(path) => {
                // There should never be duplicate "start transactions" in a log
                // Every start should be followed by:
                //    * nothing (if there was an abrupt stop)
                //    * an End (if the transaction was completed)
                if !transactions.insert(path.clone()) {
                    errors.push(LogEntryError::DuplicateTransaction(path));
                }
            }
            LogEntry::EndProcessing(path) => {
                if !transactions.remove(&path) {
                    errors.push(LogEntryError::EndedMissingTransaction(path));
                }
            }
            LogEntry::UnrecoverableError => return Err(ValidateLogError::UnrecoverableError),
        }
    }
    for transaction in transactions {
        errors.push(LogEntryError::UnfinishedTransaction(transaction));
    }
    if !errors.is_empty() {
        return Err(ValidateLogError::EntryErrors(errors));
    }
    Ok(())
}

/// A transaction log factory that uses a file as its storage.
pub struct FileTransactionLogFactory {
    /// The file path that the transaction log should write to.
    pub file_path: PathBuf,
}

const LOG_PATH: &str = "imported_assets/log";

impl Default for FileTransactionLogFactory {
    fn default() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let base_path = crate::io::file::get_base_path();
        #[cfg(target_arch = "wasm32")]
        let base_path = PathBuf::new();
        let file_path = base_path.join(LOG_PATH);
        Self { file_path }
    }
}

impl ProcessorTransactionLogFactory for FileTransactionLogFactory {
    fn read(&self) -> BoxedFuture<'_, Result<Vec<LogEntry>, BevyError>> {
        let path = self.file_path.clone();
        Box::pin(async move {
            let mut log_lines = Vec::new();
            let mut file = match File::open(path).await {
                Ok(file) => file,
                Err(err) => {
                    if err.kind() == futures_io::ErrorKind::NotFound {
                        // if the log file doesn't exist, this is equivalent to an empty file
                        return Ok(log_lines);
                    }
                    return Err(err.into());
                }
            };
            let mut string = String::new();
            file.read_to_string(&mut string).await?;
            for line in string.lines() {
                if let Some(path_str) = line.strip_prefix(ENTRY_BEGIN) {
                    log_lines.push(LogEntry::BeginProcessing(
                        AssetPath::parse(path_str).into_owned(),
                    ));
                } else if let Some(path_str) = line.strip_prefix(ENTRY_END) {
                    log_lines.push(LogEntry::EndProcessing(
                        AssetPath::parse(path_str).into_owned(),
                    ));
                } else if line.is_empty() {
                    continue;
                } else {
                    return Err(ReadLogError::InvalidLine(line.to_string()).into());
                }
            }
            Ok(log_lines)
        })
    }

    fn create_new_log(
        &self,
    ) -> BoxedFuture<'_, Result<Box<dyn ProcessorTransactionLog>, BevyError>> {
        let path = self.file_path.clone();
        Box::pin(async move {
            match async_fs::remove_file(&path).await {
                Ok(_) => { /* successfully removed file */ }
                Err(err) => {
                    // if the log file is not found, we assume we are starting in a fresh (or good) state
                    if err.kind() != futures_io::ErrorKind::NotFound {
                        error!("Failed to remove previous log file {}", err);
                    }
                }
            }

            if let Some(parent_folder) = path.parent() {
                async_fs::create_dir_all(parent_folder).await?;
            }

            Ok(Box::new(FileProcessorTransactionLog {
                log_file: File::create(path).await?,
            }) as _)
        })
    }
}

/// A "write ahead" logger that helps ensure asset importing is transactional.
///
/// Prior to processing an asset, we write to the log to indicate it has started
/// After processing an asset, we write to the log to indicate it has finished.
/// On startup, the log can be read to determine if any transactions were incomplete.
struct FileProcessorTransactionLog {
    /// The file to write logs to.
    log_file: File,
}

impl FileProcessorTransactionLog {
    /// Write `line` to the file and flush it.
    async fn write(&mut self, line: &str) -> Result<(), BevyError> {
        self.log_file.write_all(line.as_bytes()).await?;
        self.log_file.flush().await?;
        Ok(())
    }
}

const ENTRY_BEGIN: &str = "Begin ";
const ENTRY_END: &str = "End ";
const UNRECOVERABLE_ERROR: &str = "UnrecoverableError";

impl ProcessorTransactionLog for FileProcessorTransactionLog {
    fn begin_processing<'a>(
        &'a mut self,
        asset: &'a AssetPath<'_>,
    ) -> BoxedFuture<'a, Result<(), BevyError>> {
        Box::pin(async move { self.write(&format!("{ENTRY_BEGIN}{asset}\n")).await })
    }

    fn end_processing<'a>(
        &'a mut self,
        asset: &'a AssetPath<'_>,
    ) -> BoxedFuture<'a, Result<(), BevyError>> {
        Box::pin(async move { self.write(&format!("{ENTRY_END}{asset}\n")).await })
    }

    fn unrecoverable(&mut self) -> BoxedFuture<'_, Result<(), BevyError>> {
        Box::pin(async move { self.write(UNRECOVERABLE_ERROR).await })
    }
}

/// An error that occurs when reading from the [`ProcessorTransactionLog`] fails.
#[derive(Error, Debug)]
pub enum ReadLogError {
    /// An invalid log line was encountered, consisting of the contained string.
    #[error("Encountered an invalid log line: '{0}'")]
    InvalidLine(String),
    /// A file-system-based error occurred while reading the log file.
    #[error("Failed to read log file: {0}")]
    Io(#[from] futures_io::Error),
}

/// An error that occurs when writing to the [`ProcessorTransactionLog`] fails.
#[derive(Error, Debug)]
#[error(
    "Failed to write {log_entry:?} to the asset processor log. This is not recoverable. {error}"
)]
pub(crate) struct WriteLogError {
    pub(crate) log_entry: LogEntry,
    pub(crate) error: BevyError,
}

/// An error that occurs when validating the [`ProcessorTransactionLog`] fails.
#[derive(Error, Debug)]
pub enum ValidateLogError {
    /// An error that could not be recovered from. All assets will be reprocessed.
    #[error("Encountered an unrecoverable error. All assets will be reprocessed.")]
    UnrecoverableError,
    /// A [`ReadLogError`].
    #[error("Failed to read log entries: {0}")]
    ReadLogError(BevyError),
    /// Duplicated process asset transactions occurred.
    #[error("Encountered a duplicate process asset transaction: {0:?}")]
    EntryErrors(Vec<LogEntryError>),
}

/// An error that occurs when validating individual [`ProcessorTransactionLog`] entries.
#[derive(Error, Debug)]
pub enum LogEntryError {
    /// A duplicate process asset transaction occurred for the given asset path.
    #[error("Encountered a duplicate process asset transaction: {0}")]
    DuplicateTransaction(AssetPath<'static>),
    /// A transaction was ended that never started for the given asset path.
    #[error("A transaction was ended that never started {0}")]
    EndedMissingTransaction(AssetPath<'static>),
    /// An asset started processing but never finished at the given asset path.
    #[error("An asset started processing but never finished: {0}")]
    UnfinishedTransaction(AssetPath<'static>),
}
