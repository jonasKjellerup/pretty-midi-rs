use std::collections::{HashMap, VecDeque};
use midly::{MetaMessage, MidiMessage, TrackEvent};

struct Instrument {
    program: (),
    is_drum: bool,
    name: String,
    notes: Vec<Note>,
    pitch_bends: Vec<()>,
    control_changes: Vec<()>,
}

struct Note {
    pitch: u8,
    velocity: u8,

    step: u32,
    start_time: u32,
    end_time: u32,
}

struct ChannelState {
    flags: u128,
    active_notes: [Vec<(u16, u8)>;128],
}

impl ChannelState {
    fn apply_midi_msg(&mut self, time: u16, msg: &MidiMessage) {
        match msg {
            | MidiMessage::NoteOn { key, vel } if vel.as_int() > 0
                => self.note_on(time, key.as_int(), vel.as_int()),

            | MidiMessage::NoteOff { key, .. }
            | MidiMessage::NoteOn { key, ..}
                => self.note_off(time, key.as_int()),

            | MidiMessage::ProgramChange {..}
            | MidiMessage::PitchBend {..}
            | MidiMessage::Controller {..} => todo!(),

            _ => panic!("Unexpected MIDI message type.")
        }
    }

    fn note_on(&mut self, time: u16, key: u8, vel: u8) {
        let mask: u128 = 1 << key;
        self.flags |= mask;

        self.active_notes[key as usize].push((time, vel));
    }

    fn note_off(&mut self, time: u16, key: u8) {
        let mask: u128 = 1 << key;
        if (self.flags & mask) > 0 {

        }
    }
}

struct TrackState {
    channels: [ChannelState;16],
    instrument_map: HashMap<(u8, u8), u32>,
}

impl TrackState {
    fn apply_midi_event(&mut self, event: &midly::TrackEvent) {}
}

fn read_midi_file(path: &str) {}

fn get_header_values(smf: midly::Smf) -> (u16, ) {
    match smf.header.timing {
        midly::Timing::Metrical(t) => (t.as_int(), ),
        _ => panic!("Non metrical timing not supported.")
    }
}

fn as_tempo_change(event: &TrackEvent) -> Option<(u32, u32)> {
    match event.kind {
        midly::TrackEventKind::Meta(MetaMessage::Tempo(x))
        => Some((event.delta.as_int(), x.as_int())),
        _ => None,
    }
}

fn generate_tick_scales(track: &midly::Track, resolution: u16) -> VecDeque<(u32, f32)> {
    let resolution = resolution as f32;
    let mut last_tick_scale = -1.0;

    let mut scales: VecDeque<_> = track.into_iter()
        .filter_map(as_tempo_change)
        .filter_map(|(time, tempo)| {
            let tick_scale = 60.0 / ((6e7 / tempo as f32) * resolution);
            if tick_scale != last_tick_scale {
                Some((time, tick_scale))
            } else {
                None
            }
        })
        .collect();


    let missing_initial_scale = scales.front()
        .map(|(time, _)| *time > 0)
        .unwrap_or(false);

    if missing_initial_scale {
        scales.push_front((0, 60.0 / (120.0 * resolution)));
    }

    scales
}

fn make_track_time_absolute(track: midly::Track) -> midly::Track {
    let mut time = 0;
    track.into_iter()
        .map(|event| {
            time += event.delta.as_int();
            TrackEvent {
                delta: time.into(),
                ..event
            }
        })
        .collect()
}

fn get_max_tick<'l>(tracks: &'l impl Iterator<Item=midly::Track<'l>>) -> u32 {
    const MAX_TICK: u32 = 10_000_000;
    /*
        The original code finds max by iterating over all events
        max([max([ e.time for e in t] )
                            for t in midi_data.tracks]) + 1
        but since we converted all tracks to absolute time we can
        simply take the last event of each track
     */

    1 + tracks
        .map(
            |t| t.last()
                .expect("all MIDI tracks should be non empty")
                .delta.as_int()
        )
        .max()
        .unwrap()

    /*
    1 + tracks.flatten()
        .map(|event| event.delta.as_int())
        .max().unwrap_or(0)
     */
}
