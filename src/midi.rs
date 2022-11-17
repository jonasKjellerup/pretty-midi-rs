use arrayvec::ArrayVec;
use midly::{MetaMessage, MidiMessage, TrackEvent, TrackEventKind};
#[cfg(test)]
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::mem;

#[cfg(test)]
mod test;

pub trait InspectMutExt: Sized {
    type Inner;

    fn inspect_mut(self, f: impl FnOnce(&mut Self::Inner)) -> Self;
}

impl<T> InspectMutExt for Option<T> {
    type Inner = T;

    fn inspect_mut(self, f: impl FnOnce(&mut Self::Inner)) -> Self {
        self.map(|mut v| {
            f(&mut v);
            v
        })
    }
}

const DEFAULT_TEMPO: u16 = 50000;
const DEFAULT_TICKS_PER_BEAT: u16 = 480;

type ProgramNo = u8;
type ChannelNo = u8;
type ControlNo = u8;

type ControlValue = u8;
type Pitch = u8;
type PitchBendValue = u16;
type Velocity = u8;
type MidiTime = u32;

pub trait TimeUnit {
    type Repr: std::fmt::Debug;
}

#[derive(Debug, PartialEq, Eq)]
pub struct RealTime;
impl TimeUnit for RealTime {
    type Repr = f32;
}

#[derive(Debug, PartialEq, Eq)]
pub struct TickTime;
impl TimeUnit for TickTime {
    type Repr = MidiTime;
}

#[cfg_attr(test, derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Debug)]
struct Note<T: TimeUnit> {
    pitch: Pitch,
    velocity: Velocity,

    #[cfg_attr(test, serde(alias = "start"))]
    start_time: T::Repr,
    #[cfg_attr(test, serde(alias = "end"))]
    end_time: T::Repr,
}

#[derive(Debug)]
struct PitchBend {
    bend: PitchBendValue,
    time: MidiTime,
}

#[derive(Debug)]
struct ControlChange {
    number: ControlNo,
    value: ControlValue,
    time: MidiTime,
}

type TickScale = (u32, f32);

fn as_tempo_change(event: &TrackEvent) -> Option<TickScale> {
    match event.kind {
        TrackEventKind::Meta(MetaMessage::Tempo(x)) => Some((event.delta.as_int(), x.as_int() as f32)),
        _ => None,
    }
}

fn generate_tick_scales(track: &midly::Track, resolution: u16) -> VecDeque<TickScale> {
    let resolution = resolution as f32;
    let mut last_tick_scale = -1.0;

    let mut scales: VecDeque<_> = track
        .into_iter()
        .filter_map(as_tempo_change)
        .filter_map(|(time, tempo)| {
            let tick_scale = 60.0 / ((6e7 / tempo) * resolution);
            if tick_scale != last_tick_scale {
                last_tick_scale = tick_scale;
                Some((time, tick_scale))
            } else {
                None
            }
        })
        .collect();

    let missing_initial_scale = scales.front().map(|(time, _)| *time > 0).unwrap_or(false);

    if missing_initial_scale {
        scales.push_front((0, 60.0 / (120.0 * resolution)));
    }

    scales
}

#[derive(Debug)]
pub struct Instrument<T: TimeUnit> {
    program: ProgramNo,
    name: String,
    notes: Vec<Note<T>>,
    pitch_bends: Vec<PitchBend>,
    control_changes: Vec<ControlChange>,
}

impl<T: TimeUnit> Instrument<T> {
    fn new(program: ProgramNo) -> Self {
        Instrument {
            program,
            name: String::new(),
            notes: vec![],
            pitch_bends: vec![],
            control_changes: vec![],
        }
    }

    fn is_drum(&self) -> bool {
        self.program == 9
    }
}

impl Instrument<TickTime> {
    /// Converts an instrument with time meassured in ticks
    /// into an instrument with time meassure into real time.
    fn to_real_time(self, scales: &[TickScale]) -> Instrument<RealTime> {
        let time_thresholds = scales
            .iter()
            .skip(1)
            .map(|(time, _)| *time)
            .chain(std::iter::once(u32::MAX));
        let scales = scales
            .iter()
            .take(scales.len())
            .map(|(_, scale)| *scale);

        let mut scales = time_thresholds.zip(scales);
        
        let mut current_scale = scales
            .next()
            .expect("There has to be atleast on scale to convert from tick time to real time.");

        let mut last_end = (0, 0f32);
        let mut last = (0, 0f32);
        let notes = self.notes.into_iter().map(|note| {
            while note.start_time > current_scale.0 {
                current_scale = scales.next().unwrap();
                last_end = last;
            }

            let real_offset = last_end.1;
            let new_ticks_start = (note.start_time - last_end.0) as f32;
            // TODO idk if it is correct to convert end time her
            let new_ticks_end = (note.end_time - last_end.0) as f32;

            Note::<RealTime> {
                start_time: new_ticks_start * current_scale.1 + real_offset,
                end_time: new_ticks_end * current_scale.1 + real_offset,
                ..note
            }
        }).collect();

        Instrument { notes, ..self }
    }
}

#[derive(Default)]
struct ChannelState {
    active_notes: ArrayVec<Vec<(MidiTime, Velocity)>, 128>,
    current_program: ProgramNo,

    straggler_notes: Option<Box<Instrument<TickTime>>>,
    instruments: HashMap<ProgramNo, Box<Instrument<TickTime>>>,
}

impl ChannelState {
    fn create_instrument(&mut self, program: ProgramNo) -> &mut Instrument<TickTime> {
        self.straggler_notes
            .take()
            .or_else(|| Some(Box::new(Instrument::new(program))))
            .and_then(|instrument| {
                self.instruments.insert(program, instrument);
                self.instruments.get_mut(&program)
            })
            .map(|v| v.as_mut())
            .unwrap()
    }

    fn get_or_create_instrument_mut(&mut self, program: ProgramNo) -> &mut Instrument<TickTime> {
        if self.instruments.contains_key(&program) {
            self.instruments.get_mut(&program).unwrap()
        } else {
            self.create_instrument(program)
        }
    }

    /// Gets the currently active instrument. Instrument selection priority:
    /// `straggler > instruments[current_program] > new straggler`
    fn current_instrument_mut(&mut self) -> &mut Instrument<TickTime> {
        if let Some(ref mut inst) = self.straggler_notes {
            inst
        } else {
            self.instruments
                .get_mut(&self.current_program)
                .or_else(|| {
                    // We create an instrument for storing straggler notes
                    // if one does exists and an instrument for the current program
                    // also does not exist.
                    self.straggler_notes = Some(Box::new(Instrument::new(0)));
                    self.straggler_notes.as_mut()
                })
                .unwrap()
        }
    }

    fn note_on(&mut self, time: MidiTime, key: u8, vel: u8) {
        self.active_notes[key as usize].push((time, vel));
    }

    fn note_off(&mut self, time: MidiTime, key: u8) {
        if self.active_notes[key as usize].len() > 0 {
            // We move the note list out of the instrument
            // to avoid mutable double borrowing
            let mut notes = mem::take(&mut self.current_instrument_mut().notes);

            self.active_notes[key as usize]
                .drain_filter(|(start, _)| *start != time)
                .map(|(start, velocity)| Note {
                    pitch: key,
                    start_time: start,
                    end_time: time,
                    velocity,
                })
                .collect_into(&mut notes);

            self.current_instrument_mut().notes = notes;
        }
    }

    fn pitch_bend(&mut self, bend: PitchBendValue, time: MidiTime) {
        let instrument = self.current_instrument_mut();
        instrument.pitch_bends.push(PitchBend { bend, time });
    }

    fn control_change(&mut self, number: ControlNo, value: ControlValue, time: MidiTime) {
        let instrument = self.current_instrument_mut();
        instrument.control_changes.push(ControlChange {
            number,
            value,
            time,
        })
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct InstrumentID {
    channel: u8,
    program: u8,
}

impl InstrumentID {
    fn new(channel: u8, program: u8) -> Self {
        InstrumentID { channel, program }
    }
}

#[derive(Default)]
struct TrackState {
    name: Option<String>,
    channels: [ChannelState; 16],
    instruments: HashMap<InstrumentID, Instrument<TickTime>>,
}

impl TrackState {
    fn init_channels(&mut self) {
        for channel in &mut self.channels {
            while channel.active_notes.remaining_capacity() > 0 {
                channel.active_notes.push(vec![])
            }
        }
    }
    
    fn apply_event(&mut self, event: &TrackEvent) {
        let time = event.delta.as_int();
        match event.kind {
            TrackEventKind::Midi {
                channel,
                ref message,
            } => self.apply_midi_msg(channel.as_int(), message, time),

            TrackEventKind::Meta(ref msg) => self.apply_meta_msg(msg),

            TrackEventKind::SysEx(_) | TrackEventKind::Escape(_) => {}
        }
    }

    fn apply_midi_msg(&mut self, channel: ChannelNo, msg: &MidiMessage, time: MidiTime) {
        match msg {
            MidiMessage::ProgramChange { program } => {
                self.channels[channel as usize].current_program = program.as_int()
            }

            MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => self
                .get_channel_mut(channel)
                .note_on(time, key.as_int(), vel.as_int()),

            MidiMessage::NoteOff { key, .. } | MidiMessage::NoteOn { key, .. } => {
                self.get_channel_mut(channel).note_off(time, key.as_int())
            }

            MidiMessage::PitchBend {
                bend: midly::PitchBend(bend),
            } => self
                .get_channel_mut(channel)
                .pitch_bend(bend.as_int(), time),

            MidiMessage::Controller { controller, value } => self
                .get_channel_mut(channel)
                .control_change(controller.as_int(), value.as_int(), time),

            // pretty-midi ignores these, so we do the same
            MidiMessage::Aftertouch { .. } | MidiMessage::ChannelAftertouch { .. } => (),
        }
    }

    fn apply_meta_msg(&mut self, msg: &MetaMessage) {
        match msg {
            MetaMessage::TrackName(name) => {
                self.name = Some(String::from_utf8_lossy(name).into_owned());
            }

            MetaMessage::InstrumentName(_) => {}

            MetaMessage::TrackNumber(..) => {}
            MetaMessage::Text(..) => {}
            MetaMessage::Copyright(..) => {}
            MetaMessage::Lyric(..) => {}
            MetaMessage::Marker(..) => {}
            MetaMessage::CuePoint(..) => {}
            MetaMessage::ProgramName(..) => {}
            MetaMessage::DeviceName(..) => {}
            MetaMessage::MidiChannel(..) => {}
            MetaMessage::MidiPort(..) => {}
            MetaMessage::EndOfTrack => {}
            MetaMessage::Tempo(..) => {}
            MetaMessage::SmpteOffset(..) => {}
            MetaMessage::TimeSignature(..) => {}
            MetaMessage::KeySignature(..) => {}
            MetaMessage::SequencerSpecific(..) => {}
            MetaMessage::Unknown(..) => {}
        }
    }

    fn get_channel_mut(&mut self, channel: ChannelNo) -> &mut ChannelState {
        &mut self.channels[channel as usize]
    }
}

fn read_midi_file(path: &str) {}

fn get_timing(smf: &midly::Smf) -> u16 {
    match smf.header.timing {
        midly::Timing::Metrical(t) => t.as_int(),
        _ => panic!("Non metrical timing not supported."),
    }
}

fn make_track_time_absolute(track: midly::Track) -> midly::Track {
    let mut time = 0;
    track
        .into_iter()
        .map(|event| {
            time += event.delta.as_int();
            TrackEvent {
                delta: time.into(),
                ..event
            }
        })
        .collect()
}

fn get_max_tick<'l>(tracks: impl Iterator<Item = &'l midly::Track<'l>>) -> u32 {
    const MAX_TICK: u32 = 10_000_000;
    /*
       The original code finds max by iterating over all events
       max([max([ e.time for e in t] )
                           for t in midi_data.tracks]) + 1
       but since we converted all tracks to absolute time we can
       simply take the last event of each track
    */

    1 + tracks
        .map(|t| {
            t.last()
                .expect("all MIDI tracks should be non empty")
                .delta
                .as_int()
        })
        .max()
        .unwrap()

    /*
    1 + tracks.flatten()
        .map(|event| event.delta.as_int())
        .max().unwrap_or(0)
     */
}

struct MidiReader<'l> {
    smf: &'l midly::Smf<'l>,
    track_state: Vec<TrackState>,
    track_offset: usize,
}

impl<'l> MidiReader<'l> {
    fn new(src: &'l midly::Smf<'l>) -> Self {
        let midly::Format::Parallel = src.header.format else {
            panic!("SMF formats other than parallel (format 1) are not currently supported1");
        };

        let track_count = src.tracks.len();
        MidiReader {
            smf: src,
            track_state: Vec::with_capacity(track_count - 1),
            track_offset: 1,
        }
    }

    fn build_track_state(&mut self) {
        self.smf
            .tracks
            .iter()
            .skip(self.track_offset)
            .map(|track| {
                let mut track_state = TrackState::default();
                track_state.init_channels();
                for event in track {
                    track_state.apply_event(event);
                }
                track_state
            })
            .collect_into(&mut self.track_state);
    }

    pub fn build_instrument_data(&mut self) -> Vec<Instrument<RealTime>> {
        if self.track_state.len() == 0 {
            self.build_track_state();
        }

        let scales: Vec<_> = generate_tick_scales(&self.smf.tracks[0], DEFAULT_TICKS_PER_BEAT).into();
        
        dbg!(self.track_state.len());
        dbg!(self.track_state[0].instruments.len());
        
        self.track_state[0].channels.iter().for_each(|channel| {
            dbg!(channel.active_notes.len());
            channel.straggler_notes.as_ref().map(|n| {
                dbg!(n.program);
                dbg!(n.notes.len());
                dbg!(&n.name);
            });
            dbg!(channel.instruments.len());
            dbg!(channel.current_program);
        });
        
        self.track_state
            .iter_mut()
            .flat_map(|state| {
                state
                    .instruments
                    .drain()
                    .map(|(_, v)| v.to_real_time(scales.as_ref()))
            })
            .collect()
    }
}
