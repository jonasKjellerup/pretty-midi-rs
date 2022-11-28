from pretty_midi_rs import MidiObject
import pretty_midi

a_data = MidiObject("./test_data/source.mid")
b_data = pretty_midi.PrettyMIDI("./test_data/source.mid")

def comp_float(a: float, b: float) -> bool:
    # we consider them identical if they are within a milisecond of each other
    return abs(a - b) < 0.001

for (note_a, note_b) in zip(a_data.instruments[0].notes, b_data.instruments[0].notes):
    if note_a.pitch == note_b.pitch\
        and comp_float(note_a.start, note_b.start)\
        and comp_float(note_a.end, note_b.end)\
        and note_a.velocity == note_b.velocity:
        continue
    else:
        print("Notes are not equal")
        print(f"pitch {note_a.pitch} == {note_b.pitch}")
        print(f"velocity {note_a.velocity} == {note_b.velocity}")
        print(f"start {note_a.start} == {note_b.start} -> {note_a.start - note_b.start}")
        print(f"end {note_a.end} == {note_b.end} -> {note_a.start - note_b.start}")
        break
    
