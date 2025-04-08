use core::fmt::{Debug, Display};

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

/// An error associated with an IO operation.
pub struct Error {
    kind: ErrorKind,
    inner: InnerError,
}

enum InnerError {
    Simple,
    Code(RawOsError),
    #[cfg(feature = "alloc")]
    Complex(Box<dyn core::error::Error + Send + Sync>),
}

impl core::error::Error for Error {}

impl Debug for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <ErrorKind as Debug>::fmt(&self.kind, fmt)?;
        match &self.inner {
            InnerError::Simple => Ok(()),
            InnerError::Code(code) => {
                fmt.write_str(": ")?;
                <RawOsError as Debug>::fmt(code, fmt)
            }
            #[cfg(feature = "alloc")]
            InnerError::Complex(error) => {
                fmt.write_str(": ")?;
                <Box<dyn core::error::Error + Send + Sync> as Debug>::fmt(error, fmt)
            }
        }
    }
}

impl Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <ErrorKind as Display>::fmt(&self.kind, fmt)?;
        match &self.inner {
            InnerError::Simple => Ok(()),
            InnerError::Code(code) => {
                fmt.write_str(": ")?;
                <RawOsError as Display>::fmt(code, fmt)
            }
            #[cfg(feature = "alloc")]
            InnerError::Complex(error) => {
                fmt.write_str(": ")?;
                <Box<dyn core::error::Error + Send + Sync> as Display>::fmt(error, fmt)
            }
        }
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error {
            kind,
            inner: InnerError::Simple,
        }
    }
}

impl Error {
    /// Gets the [kind](ErrorKind) of this error.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Gets the last reported [`Error`] from the underlying OS.
    pub fn last_os_error() -> Error {
        ErrorKind::Other.into()
    }

    /// Creates an [`Error`] from the provided [code](RawOsError).
    pub fn from_raw_os_error(code: RawOsError) -> Error {
        Error {
            kind: ErrorKind::Other,
            inner: InnerError::Code(code),
        }
    }

    /// Gets the underlying error [code](RawOsError), if applicable.
    pub fn raw_os_error(&self) -> Option<RawOsError> {
        match &self.inner {
            InnerError::Code(code) => Some(*code),
            _ => None,
        }
    }

    /// Downcasts the underlying [error](core::error::Error) into the type `E`, if applicable.
    pub fn downcast<E>(self) -> core::result::Result<E, Self>
    where
        E: core::error::Error + Send + Sync + 'static,
    {
        match self.inner {
            #[cfg(feature = "alloc")]
            InnerError::Complex(error) if error.as_ref().is::<E>() => {
                let res = error.downcast::<E>();
                Ok(*res.unwrap())
            }
            inner => Err(Self {
                kind: self.kind,
                inner,
            }),
        }
    }

    /// Gets a reference to the underlying [error](core::error::Error), if applicable.
    pub fn get_ref(&self) -> Option<&(dyn core::error::Error + Send + Sync)> {
        match &self.inner {
            #[cfg(feature = "alloc")]
            InnerError::Complex(error) => Some(error.as_ref()),
            _ => None,
        }
    }

    /// Gets a mutable reference to the underlying [error](core::error::Error), if applicable.
    pub fn get_mut(&mut self) -> Option<&mut (dyn core::error::Error + Send + Sync)> {
        match &mut self.inner {
            #[cfg(feature = "alloc")]
            InnerError::Complex(error) => Some(error.as_mut()),
            _ => None,
        }
    }
}

#[cfg(feature = "alloc")]
impl Error {
    /// Creates a new [`Error`] based on the provided [kind](ErrorKind) and inner [error](core::error::Error).
    pub fn new<E>(kind: ErrorKind, error: E) -> Error
    where
        E: Into<Box<dyn core::error::Error + Send + Sync>>,
    {
        Error {
            kind,
            inner: InnerError::Complex(error.into()),
        }
    }

    /// Creates a new [`Error`] from the inner [error](core::error::Error).
    pub fn other<E>(error: E) -> Error
    where
        E: Into<Box<dyn core::error::Error + Send + Sync>>,
    {
        Error::new(ErrorKind::Other, error)
    }

    /// Gets the underlying [error](core::error::Error), if applicable.
    pub fn into_inner(self) -> Option<Box<dyn core::error::Error + Send + Sync>> {
        match self.inner {
            InnerError::Complex(error) => Some(error),
            _ => None,
        }
    }
}

/// A classification of IO [`Error`].
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ErrorKind {
    /// An entity was not found, often a file.
    NotFound,
    /// The operation lacked the necessary privileges to complete.
    PermissionDenied,
    /// The connection was refused by the remote server.
    ConnectionRefused,
    /// The connection was reset by the remote server.
    ConnectionReset,
    /// The remote host is not reachable.
    HostUnreachable,
    /// The network containing the remote host is not reachable.
    NetworkUnreachable,
    /// The connection was aborted (terminated) by the remote server.
    ConnectionAborted,
    /// The network operation failed because it was not connected yet.
    NotConnected,
    /// A socket address could not be bound because the address is already in
    /// use elsewhere.
    AddrInUse,
    /// A nonexistent interface was requested or the requested address was not
    /// local.
    AddrNotAvailable,
    /// The system's networking is down.
    NetworkDown,
    /// The operation failed because a pipe was closed.
    BrokenPipe,
    /// An entity already exists, often a file.
    AlreadyExists,
    /// The operation needs to block to complete, but the blocking operation was
    /// requested to not occur.
    WouldBlock,
    /// A filesystem object is, unexpectedly, not a directory.
    NotADirectory,
    /// The filesystem object is, unexpectedly, a directory.
    IsADirectory,
    /// A non-empty directory was specified where an empty directory was expected.
    DirectoryNotEmpty,
    /// The filesystem or storage medium is read-only, but a write operation was attempted.
    ReadOnlyFilesystem,
    /// Stale network file handle.
    StaleNetworkFileHandle,
    /// A parameter was incorrect.
    InvalidInput,
    /// Data not valid for the operation were encountered.
    InvalidData,
    /// The I/O operation's timeout expired, causing it to be canceled.
    TimedOut,
    /// An error returned when an operation could not be completed because a
    /// call to [`write`] returned [`Ok(0)`].
    WriteZero,
    /// The underlying storage (typically, a filesystem) is full.
    StorageFull,
    /// Seek on unseekable file.
    NotSeekable,
    /// Filesystem quota or some other kind of quota was exceeded.
    QuotaExceeded,
    /// File larger than allowed or supported.
    FileTooLarge,
    /// Resource is busy.
    ResourceBusy,
    /// Executable file is busy.
    ExecutableFileBusy,
    /// Deadlock (avoided).
    Deadlock,
    /// Cross-device or cross-filesystem (hard) link or rename.
    CrossesDevices,
    /// Too many (hard) links to the same filesystem object.
    TooManyLinks,
    /// A filename was invalid.
    InvalidFilename,
    /// Program argument list too long.
    ArgumentListTooLong,
    /// This operation was interrupted.
    Interrupted,
    /// This operation is unsupported on this platform.
    Unsupported,
    /// An error returned when an operation could not be completed because an
    /// "end of file" was reached prematurely.
    UnexpectedEof,
    /// An operation could not be completed, because it failed
    /// to allocate enough memory.
    OutOfMemory,
    /// A custom error that does not fall under any other I/O error kind.
    Other,
}

impl ErrorKind {
    fn as_str(&self) -> &'static str {
        use ErrorKind::*;
        match *self {
            AddrInUse => "address in use",
            AddrNotAvailable => "address not available",
            AlreadyExists => "entity already exists",
            ArgumentListTooLong => "argument list too long",
            BrokenPipe => "broken pipe",
            ConnectionAborted => "connection aborted",
            ConnectionRefused => "connection refused",
            ConnectionReset => "connection reset",
            CrossesDevices => "cross-device link or rename",
            Deadlock => "deadlock",
            DirectoryNotEmpty => "directory not empty",
            ExecutableFileBusy => "executable file busy",
            FileTooLarge => "file too large",
            HostUnreachable => "host unreachable",
            Interrupted => "operation interrupted",
            InvalidData => "invalid data",
            InvalidFilename => "invalid filename",
            InvalidInput => "invalid input parameter",
            IsADirectory => "is a directory",
            NetworkDown => "network down",
            NetworkUnreachable => "network unreachable",
            NotADirectory => "not a directory",
            NotConnected => "not connected",
            NotFound => "entity not found",
            NotSeekable => "seek on unseekable file",
            Other => "other error",
            OutOfMemory => "out of memory",
            PermissionDenied => "permission denied",
            QuotaExceeded => "quota exceeded",
            ReadOnlyFilesystem => "read-only filesystem or storage medium",
            ResourceBusy => "resource busy",
            StaleNetworkFileHandle => "stale network file handle",
            StorageFull => "no storage space",
            TimedOut => "timed out",
            TooManyLinks => "too many links",
            UnexpectedEof => "unexpected end of file",
            Unsupported => "unsupported",
            WouldBlock => "operation would block",
            WriteZero => "write zero",
        }
    }
}

impl Display for ErrorKind {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fmt.write_str(self.as_str())
    }
}

/// A result returned from IO operations.
pub type Result<T> = core::result::Result<T, Error>;

/// A raw error code.
pub type RawOsError = i32;
