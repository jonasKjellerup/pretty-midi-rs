use std::error::Error;

use crate::midi::MidiReader;

const NOTE_DATA: &'static [u8] = include_bytes!("../../test_data/notes.json");
const SCALE_DATA: &'static [u8] = include_bytes!("../../test_data/scales.json");
const MIDI_DATA: &'static [u8] = include_bytes!("../../test_data/source.mid");

const TOLERANCE: f32 = 0.000001;

fn compare_f32(a: f32, b: f32) {
    assert!(a == b || a.abs_sub(b) < TOLERANCE)
}

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn test_instrument_scanning() -> TestResult {
    let source_data: Vec<super::Note<super::RealTime>> = serde_json::from_slice(NOTE_DATA)?;
    let mut smf = midly::Smf::parse(MIDI_DATA)?;
    
    smf.tracks = smf.tracks.into_iter().map(super::make_track_time_absolute).collect();

    let mut reader = MidiReader::new(&smf);
    let instruments = reader.build_instrument_data();

    assert_eq!(instruments.len(), 1);

    let notes = &instruments[0].notes;

    assert_eq!(notes.len(), source_data.len());

    source_data.iter()
        .zip(notes.iter())
        .for_each(|(reference, generated)| {
            assert_eq!(reference.pitch, generated.pitch);
            assert_eq!(reference.velocity, generated.velocity);
            compare_f32(reference.start_time, generated.start_time);
            compare_f32(reference.end_time, generated.end_time);
        });

    Ok(())
}

#[test]
fn test_tick_scale_computation() -> TestResult {
    let reference_values: Vec<super::TickScale> = serde_json::from_slice(SCALE_DATA)?;
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
