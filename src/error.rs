use core::fmt;

/// Error code from TJpgDec
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JpegError {
    /// Succeeded
    Ok = 0,
    /// Interrupted by output function
    Interrupted,
    /// Device error or wrong termination of input stream
    InputError,
    /// Insufficient memory pool for the image
    InsufficientMemoryPool,
    /// Insufficient stream input buffer
    InsufficientStreamBuffer,
    /// Parameter error
    ParameterError,
    /// Data format error (may be broken data)
    DataFormatError,
    /// Right format but not supported
    UnsupportedFormat,
    /// Not supported JPEG standard
    UnsupportedJpegStandard,
}

impl fmt::Display for JpegError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JpegError::Ok => write!(f, "Succeeded"),
            JpegError::Interrupted => write!(f, "Interrupted by output function"),
            JpegError::InputError => write!(f, "Device error or wrong termination of input stream"),
            JpegError::InsufficientMemoryPool => write!(f, "Insufficient memory pool for the image"),
            JpegError::InsufficientStreamBuffer => write!(f, "Insufficient stream input buffer"),
            JpegError::ParameterError => write!(f, "Parameter error"),
            JpegError::DataFormatError => write!(f, "Data format error (may be broken data)"),
            JpegError::UnsupportedFormat => write!(f, "Right format but not supported"),
            JpegError::UnsupportedJpegStandard => write!(f, "Not supported JPEG standard"),
        }
    }
}
