# pretty-midi-rs

This project reimplements a subset of the python library [pretty-midi](https://github.com/craffel/pretty-midi) in Rust and exposes python bindings through PyO3.

The following features are reimplemented in this version:

 - Parsing of MIDI files
    - Conversion from MIDI events to discrete notes.
    - Conversion from MIDI ticks to seconds.