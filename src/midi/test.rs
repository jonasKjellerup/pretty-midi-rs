use std::error::Error;

use crate::midi::MidiReader;

const JSON_DATA: &'static [u8] = include_bytes!("../../test_data.json");
const MIDI_DATA: &'static [u8] = include_bytes!("../../test_data.mid");

#[test]
fn test_instrument_scanning() -> Result<(), Box<dyn Error>> {
    let source_data: Vec<super::Note> = serde_json::from_slice(JSON_DATA)?;
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
