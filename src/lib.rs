#![feature(drain_filter)]
#![feature(iter_collect_into)]

/// A Rust implementation of the python library "pretty-midi".

mod midi;

use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;
use pyo3::{
    exceptions::{PyBaseException, PyIOError},
    prelude::*,
};

// Error handling/conversion

#[derive(Debug)]
enum ErrorKind {
    IO,
    Midly,
    Generic,
}

#[derive(Debug)]
struct Error {
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

#[pymodule]
fn pretty_midi(_: Python<'_>, m: &PyModule) -> PyResult<()> {
    Ok(())
}

#[pyclass]
struct Instrument(midi::Instrument);

#[pyclass(sequence)]
struct MidiObject {
    #[pyo3(get, set)]
    resolution: u16,

    tick_scale: VecDeque<(u32, f32)>,
    instruments: Vec<midi::Instrument>,
}

#[pymethods]
impl MidiObject {
    #[new]
    fn new(file_path: Option<&str>, resolution: Option<u16>, initial_tempo: Option<u32>) -> PyResult<Self> {
        let resolution = resolution.unwrap_or(220);
        let initial_tempo = initial_tempo.unwrap_or(120);

        if let Some(path) = file_path {
            MidiObject::from_file(path)
        } else {
            Ok(MidiObject {
                resolution,
                tick_scale: VecDeque::new(),
                instruments: vec![],
            })
        }
    }
}

impl MidiObject {
    fn from_file(file_path: &str) -> PyResult<Self> {
        let mut file = File::open(file_path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let smf = midly::Smf::parse(&data)
            .map_err(Error::from)?;


        Ok(MidiObject {
            resolution: 0,
            tick_scale: VecDeque::new(),
            instruments: vec![],
        })
    }
}