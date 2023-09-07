use async_fs::File;
use bevy_log::error;
use bevy_utils::HashSet;
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// An in-memory representation of a single [`ProcessorTransactionLog`] entry.
#[derive(Debug)]
pub(crate) enum LogEntry {
    BeginProcessing(PathBuf),
    EndProcessing(PathBuf),
    UnrecoverableError,
}

/// A "write ahead" logger that helps ensure asset importing is transactional.
/// Prior to processing an asset, we write to the log to indicate it has started
/// After processing an asset, we write to the log to indicate it has finished.
/// On startup, the log can be read to determine if any transactions were incomplete.
// TODO: this should be a trait
pub struct ProcessorTransactionLog {
    log_file: File,
}

/// An error that occurs when reading from the [`ProcessorTransactionLog`] fails.
#[derive(Error, Debug)]
pub enum ReadLogError {
    #[error("Encountered an invalid log line: '{0}'")]
    InvalidLine(String),
    #[error("Failed to read log file: {0}")]
    Io(#[from] futures_io::Error),
}

/// An error that occurs when writing to the [`ProcessorTransactionLog`] fails.
#[derive(Error, Debug)]
#[error(
    "Failed to write {log_entry:?} to the asset processor log. This is not recoverable. {error}"
)]
pub struct WriteLogError {
    log_entry: LogEntry,
    error: futures_io::Error,
}

/// An error that occurs when validating the [`ProcessorTransactionLog`] fails.
#[derive(Error, Debug)]
pub enum ValidateLogError {
    #[error("Encountered an unrecoverable error. All assets will be reprocessed.")]
    UnrecoverableError,
    #[error(transparent)]
    ReadLogError(#[from] ReadLogError),
    #[error("Encountered a duplicate process asset transaction: {0:?}")]
    EntryErrors(Vec<LogEntryError>),
}

/// An error that occurs when validating individual [`ProcessorTransactionLog`] entries.
#[derive(Error, Debug)]
pub enum LogEntryError {
    #[error("Encountered a duplicate process asset transaction: {0:?}")]
    DuplicateTransaction(PathBuf),
    #[error("A transaction was ended that never started {0:?}")]
    EndedMissingTransaction(PathBuf),
    #[error("An asset started processing but never finished: {0:?}")]
    UnfinishedTransaction(PathBuf),
}

const LOG_PATH: &str = "imported_assets/log";
const ENTRY_BEGIN: &str = "Begin ";
const ENTRY_END: &str = "End ";
const UNRECOVERABLE_ERROR: &str = "UnrecoverableError";

impl ProcessorTransactionLog {
    fn full_log_path() -> PathBuf {
        #[cfg(not(target_arch = "wasm32"))]
        let base_path = crate::io::file::get_base_path();
        #[cfg(target_arch = "wasm32")]
        let base_path = PathBuf::new();
        base_path.join(LOG_PATH)
    }
    /// Create a new, fresh log file. This will delete the previous log file if it exists.
    pub(crate) async fn new() -> Result<Self, futures_io::Error> {
        let path = Self::full_log_path();
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

        Ok(Self {
            log_file: File::create(path).await?,
        })
    }

    pub(crate) async fn read() -> Result<Vec<LogEntry>, ReadLogError> {
        let mut log_lines = Vec::new();
        let mut file = match File::open(Self::full_log_path()).await {
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
                log_lines.push(LogEntry::BeginProcessing(PathBuf::from(path_str)));
            } else if let Some(path_str) = line.strip_prefix(ENTRY_END) {
                log_lines.push(LogEntry::EndProcessing(PathBuf::from(path_str)));
            } else if line.is_empty() {
                continue;
            } else {
                return Err(ReadLogError::InvalidLine(line.to_string()));
            }
        }
        Ok(log_lines)
    }

    pub(crate) async fn validate() -> Result<(), ValidateLogError> {
        let mut transactions: HashSet<PathBuf> = Default::default();
        let mut errors: Vec<LogEntryError> = Vec::new();
        let entries = Self::read().await?;
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

    /// Logs the start of an asset being processed. If this is not followed at some point in the log by a closing [`ProcessorTransactionLog::end_processing`],
    /// in the next run of the processor the asset processing will be considered "incomplete" and it will be reprocessed.
    pub(crate) async fn begin_processing(&mut self, path: &Path) -> Result<(), WriteLogError> {
        self.write(&format!("{ENTRY_BEGIN}{}\n", path.to_string_lossy()))
            .await
            .map_err(|e| WriteLogError {
                log_entry: LogEntry::BeginProcessing(path.to_owned()),
                error: e,
            })
    }

    /// Logs the end of an asset being successfully processed. See [`ProcessorTransactionLog::begin_processing`].
    pub(crate) async fn end_processing(&mut self, path: &Path) -> Result<(), WriteLogError> {
        self.write(&format!("{ENTRY_END}{}\n", path.to_string_lossy()))
            .await
            .map_err(|e| WriteLogError {
                log_entry: LogEntry::EndProcessing(path.to_owned()),
                error: e,
            })
    }

    /// Logs an unrecoverable error. On the next run of the processor, all assets will be regenerated. This should only be used as a last resort.
    /// Every call to this should be considered with scrutiny and ideally replaced with something more granular.
    pub(crate) async fn unrecoverable(&mut self) -> Result<(), WriteLogError> {
        self.write(UNRECOVERABLE_ERROR)
            .await
            .map_err(|e| WriteLogError {
                log_entry: LogEntry::UnrecoverableError,
                error: e,
            })
    }

    async fn write(&mut self, line: &str) -> Result<(), futures_io::Error> {
        self.log_file.write_all(line.as_bytes()).await?;
        self.log_file.flush().await?;
        Ok(())
    }
}
