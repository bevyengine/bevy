use async_fs::File;
use bevy_log::error;
use bevy_utils::HashSet;
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug)]
pub(crate) enum LogEntry {
    BeginPath(PathBuf),
    EndPath(PathBuf),
}

// TODO: this should be an interface
/// A "write ahead" logger that helps ensure asset importing is transactional.
/// Prior to processing an asset, we write to the log to indicate it has started
/// After processing an asset, we write to the log to indicate it has finished.
/// On startup, the log can be read to determine if any transactions were incomplete.
pub(crate) struct ProcessorTransactionLog {
    log_file: File,
}

#[derive(Error, Debug)]
pub enum ReadLogError {
    #[error("Encountered an invalid log line: '{0}'")]
    InvalidLine(String),
    #[error("Failed to read log file: {0}")]
    Io(#[from] futures_io::Error),
}

#[derive(Error, Debug)]
#[error(
    "Failed to write {log_entry:?} to the asset processor log. This is not recoverable. {error}"
)]
pub struct WriteLogError {
    log_entry: LogEntry,
    error: futures_io::Error,
}

#[derive(Error, Debug)]
pub enum ValidateLogError {
    #[error(transparent)]
    ReadLogError(#[from] ReadLogError),
    #[error("Encountered a duplicate process asset transaction: {0:?}")]
    EntryErrors(Vec<LogEntryError>),
}

#[derive(Error, Debug)]
pub enum LogEntryError {
    #[error("Encountered a duplicate process asset transaction: {0:?}")]
    DuplicateTransaction(PathBuf),
    #[error("A transaction was ended that never started {0:?}")]
    EndedMissingTransaction(PathBuf),
    #[error("An asset started processing but never finished: {0:?}")]
    UnfinishedTransaction(PathBuf),
}

const LOG_PATH: &str = ".imported_assets/log";
const ENTRY_BEGIN: &str = "Begin ";
const ENTRY_END: &str = "End ";

impl ProcessorTransactionLog {
    fn full_log_path() -> PathBuf {
        let base_path = crate::io::file::get_base_path();
        base_path.join(LOG_PATH)
    }
    /// Create a new, fresh log file. This will delete the previous log file if it exists.
    pub async fn new() -> Result<Self, futures_io::Error> {
        let path = Self::full_log_path();
        match async_fs::remove_file(&path).await {
            Ok(_) => { /* successfully removed file */ }
            Err(err) => {
                // if the log file is not found, we assume we are starting in a fresh (or good) state
                if err.kind() != futures_io::ErrorKind::NotFound {
                    error!("Failed to remove previous log file {}", err)
                }
            }
        }

        Ok(Self {
            log_file: File::create(path).await?,
        })
    }

    pub async fn read() -> Result<Vec<LogEntry>, ReadLogError> {
        let mut log_lines = Vec::new();
        let mut file = match File::open(Self::full_log_path()).await {
            Ok(file) => file,
            Err(err) => {
                if err.kind() == futures_io::ErrorKind::NotFound {
                    // if the log file doesn't exist, this is equivalent to an empty file
                    return Ok(log_lines);
                } else {
                    return Err(err.into());
                }
            }
        };
        let mut string = String::new();
        file.read_to_string(&mut string).await?;
        for line in string.lines() {
            if line.starts_with(ENTRY_BEGIN) {
                let path_str = &line[ENTRY_BEGIN.len()..];
                log_lines.push(LogEntry::BeginPath(PathBuf::from(path_str)));
            } else if line.starts_with(ENTRY_END) {
                let path_str = &line[ENTRY_END.len()..];
                log_lines.push(LogEntry::EndPath(PathBuf::from(path_str)));
            } else if line.is_empty() {
                continue;
            } else {
                return Err(ReadLogError::InvalidLine(line.to_string()));
            }
        }
        Ok(log_lines)
    }

    pub async fn validate() -> Result<(), ValidateLogError> {
        let mut transactions: HashSet<PathBuf> = Default::default();
        let mut errors: Vec<LogEntryError> = Vec::new();
        let entries = Self::read().await?;
        for entry in entries {
            match entry {
                LogEntry::BeginPath(path) => {
                    // There should never be duplicate "start transactions" in a log
                    // Every start should be followed by:
                    //    * nothing (if there was an abrupt stop)
                    //    * an End (if the transaction was completed)
                    if !transactions.insert(path.clone()) {
                        errors.push(LogEntryError::DuplicateTransaction(path));
                    }
                }
                LogEntry::EndPath(path) => {
                    if !transactions.remove(&path) {
                        errors.push(LogEntryError::EndedMissingTransaction(path));
                    }
                }
            }
        }
        for transaction in transactions {
            errors.push(LogEntryError::UnfinishedTransaction(transaction))
        }
        if !errors.is_empty() {
            return Err(ValidateLogError::EntryErrors(errors));
        }
        Ok(())
    }

    pub async fn begin_path(&mut self, path: &Path) -> Result<(), WriteLogError> {
        self.write(&format!("{ENTRY_BEGIN}{}\n", path.to_string_lossy()))
            .await
            .map_err(|e| WriteLogError {
                log_entry: LogEntry::BeginPath(path.to_owned()),
                error: e,
            })
    }

    pub async fn end_path(&mut self, path: &Path) -> Result<(), WriteLogError> {
        self.write(&format!("{ENTRY_END}{}\n", path.to_string_lossy()))
            .await
            .map_err(|e| WriteLogError {
                log_entry: LogEntry::EndPath(path.to_owned()),
                error: e,
            })
    }

    async fn write(&mut self, line: &str) -> Result<(), futures_io::Error> {
        self.log_file.write_all(line.as_bytes()).await?;
        self.log_file.flush().await?;
        Ok(())
    }
}
