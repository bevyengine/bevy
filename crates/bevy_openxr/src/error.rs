#[derive(Debug)]
pub enum Error {
    XRLoad(openxr::LoadError),
    XR(openxr::sys::Result),

    #[cfg(target_os = "android")]
    JNI(jni::errors::Error),

    Unimplemented,
}

impl From<openxr::sys::Result> for Error {
    fn from(e: openxr::sys::Result) -> Self {
        Error::XR(e)
    }
}

impl From<openxr::LoadError> for Error {
    fn from(e: openxr::LoadError) -> Self {
        Error::XRLoad(e)
    }
}

#[cfg(target_os = "android")]
impl From<jni::errors::Error> for Error {
    fn from(e: jni::errors::Error) -> Self {
        Error::JNI(e)
    }
}
