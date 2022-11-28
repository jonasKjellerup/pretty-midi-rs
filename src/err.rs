use pyo3::{
    exceptions::{PyBaseException, PyIOError},
    prelude::*,
};

#[derive(Debug)]
pub enum ErrorKind {
    IO,
    Midly,
    Generic,
}

#[derive(Debug)]
pub struct Error {
    inner: Box<dyn std::error::Error>,
    kind: ErrorKind,
}

impl From<midly::Error> for Error {
    fn from(err: midly::Error) -> Self {
        Self {
            inner: err.into(),
            kind: ErrorKind::Midly,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self {
            inner: err.into(),
            kind: ErrorKind::IO,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[evalpy err: {:?}]: {}",
            self.kind,
            self.inner.to_string()
        )
    }
}

impl std::error::Error for Error {}

impl From<Error> for PyErr {
    fn from(err: Error) -> PyErr {
        let msg = err.to_string();
        match err.kind {
            ErrorKind::IO => PyIOError::new_err(msg),
            ErrorKind::Midly | ErrorKind::Generic => PyBaseException::new_err(msg),
        }
    }
}
