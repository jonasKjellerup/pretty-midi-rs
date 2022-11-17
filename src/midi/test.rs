use std::error::Error;

use crate::midi::MidiReader;

const NOTE_DATA: &'static [u8] = include_bytes!("../../test_data/notes.json");
const SCALE_DATA: &'static [u8] = include_bytes!("../../test_data/scales.json");
const MIDI_DATA: &'static [u8] = include_bytes!("../../test_data/source.mid");

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn test_instrument_scanning() -> TestResult {
    let source_data: Vec<super::Note<super::RealTime>> = serde_json::from_slice(NOTE_DATA)?;
    let smf = midly::Smf::parse(MIDI_DATA)?;

    let mut reader = MidiReader::new(&smf);
    let instruments = reader.build_instrument_data();

    assert_eq!(instruments.len(), 1);

    let notes = &instruments[0].notes;

    assert_eq!(notes.len(), source_data.len());

    source_data.iter()
        .zip(notes.iter())
        .for_each(|(a, b)| {
            assert_eq!(a, b);
        });

    Ok(())
}

#[test]
fn test_tick_scale_computation() -> TestResult {
    let reference_values: Vec<(u32, f32)> = serde_json::from_slice(SCALE_DATA)?;
    let source_data = midly::Smf::parse(MIDI_DATA)?;
    
    let timing = super::get_timing(&source_data);
    let generated_scales = super::generate_tick_scales(&source_data.tracks[0], timing);
    
    assert_eq!(generated_scales.len(), reference_values.len());
            
    generated_scales.iter()
            .zip(reference_values.iter())
            .for_each(|(a, b)| {
                assert_eq!(a, b);
            });
            
    Ok(())
}
