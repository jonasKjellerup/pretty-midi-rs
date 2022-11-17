#!/bin/env python3

import argparse
import pretty_midi as pm
import json

parser = argparse.ArgumentParser(description='Generate test data')

parser.add_argument(
    '--input', '-i',
    dest='input', 
    default=False,
    help='determines the input midi file'
    )

parser.add_argument(
    '--output', '-o',
    dest='output', 
    default='./test_data.json',
    help='determines the output json file'
    )

args = parser.parse_args().__dict__

midi = pm.PrettyMIDI(args['input'])
notes = [x.__dict__ for x in midi.instruments[0].notes]

# Output notes
#with open(args['output'], 'w') as f:
#    json.dump(notes, f)

print(midi.resolution)

#with open("./scales.json", 'w') as f:
#    json.dump(midi._tick_scales, f)