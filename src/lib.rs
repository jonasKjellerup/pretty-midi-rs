#![feature(drain_filter)]
#![feature(iter_collect_into)]
#![feature(type_changing_struct_update)]

/// A Rust implementation of the python library "pretty-midi".

mod midi;
mod err;

use err::*;

use std::ops::Deref;
use std::{fs::File, rc::Rc};
use std::{sync::Arc};
use std::io::Read;
use pyo3::{
    exceptions::{PyBaseException, PyIOError},
    prelude::*,
};

#[derive(Clone)]
struct RcLens<T, U: 'static>(Arc<T>, &'static U);

impl<T: 'static, U: 'static> RcLens<T, U> {
    fn new<'l>(r: Arc<T>, selector: impl Fn(&'l T) -> &'l U) -> Self {
        let sr: &'static T = unsafe {std::mem::transmute(r.as_ref())};
        let image: &'static U = unsafe { std::mem::transmute(selector(sr))};
        RcLens(r, image)
    }
    
    fn map<'l, R: 'static>(self, selector: impl Fn(&'l U) -> &'l R) -> RcLens<T, R> {
        let new_image: &'static R = unsafe {std::mem::transmute(selector(self.1))};
        RcLens(self.0, new_image)
    }
}

impl<T, U> Deref for RcLens<T, U> {
    type Target = U;
    
    fn deref(&self) -> &Self::Target {
        self.1    
    }
}

#[pymodule]
fn pretty_midi_rs(_: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Instrument>()?;
    m.add_class::<MidiObject>()?;
    Ok(())
}

#[pyclass]
struct Note(RcLens<midi::Instrument<midi::RealTime>, midi::Note<midi::RealTime>>);

#[pymethods]
impl Note {
    #[getter]
    fn pitch(&self) -> u8 {
        self.0.pitch
    }
    
    #[getter]
    fn velocity(&self) -> u8 {
        self.0.velocity
    }
    
    #[getter]
    fn start(&self) -> f32 {
        self.0.start_time
    }
    
    #[getter]
    fn end(&self) -> f32 {
        self.0.end_time
    }
}

#[pyclass]
struct NoteArr(RcLens<midi::Instrument<midi::RealTime>, Vec<midi::Note<midi::RealTime>>>);

#[pymethods]
impl NoteArr {
    fn __len__(&self) -> usize {
        self.0.len()
    }
    
    fn __getitem__(&self, i: isize) -> Option<Note> {
        let i = if i < 0 {
                self.0.len() as isize + i
            } else {
                i
            } as usize;
            
        if i < self.0.len() {
            Some(Note(self.0.clone().map(|notes| &notes[i] )))
        } else {
            None
        }
    }
    
    fn __iter__(&self) -> NoteIter {
        NoteIter(self.0.clone(), 0)
    }
    
}

#[pyclass]
struct NoteIter(RcLens<midi::Instrument<midi::RealTime>, Vec<midi::Note<midi::RealTime>>>, usize);

#[pymethods]
impl NoteIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> { slf }
    
    fn __next__(&mut self) -> Option<Note> {
        if self.1 < self.0.len() {
            let note = Note(self.0.clone()
            .map(|notes| &notes[self.1] ));
            self.1 += 1;
            Some(note)
        } else {
            None
        }
    }
}

#[pyclass]
#[derive(Clone)]
struct Instrument(Arc<midi::Instrument<midi::RealTime>>);

#[pymethods]
impl Instrument {
    
    #[getter]
    fn notes(&self) -> NoteArr {
        NoteArr(RcLens::new(self.0.clone(), |instrument| &instrument.notes))
    }
    
}

#[pyclass]
struct MidiObject {
    #[pyo3(get, set)]
    resolution: u16,
    #[pyo3(get)]
    instruments: Vec<Instrument>,
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

        let mut smf = midly::Smf::parse(&data)
            .map_err(Error::from)?;

        let mut reader = midi::MidiReader::new(&mut smf);

        let instruments = reader.build_instrument_data()
            .into_iter()
            .map(|instrument| Instrument(Arc::new(instrument)))
            .collect();
        
        Ok(MidiObject {
            resolution: 0,
            instruments,
        })
    }
}