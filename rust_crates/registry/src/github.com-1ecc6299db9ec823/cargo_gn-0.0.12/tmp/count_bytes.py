#!/usr/bin/env python

import argparse

parser = argparse.ArgumentParser()
parser.add_argument("--output")
parser.add_argument('--input')
args = parser.parse_args()

out_file = open(args.output, "w+")
in_file = open(args.input, "r")

in_data = in_file.read()
in_bytes = len(in_data)

print "input bytes", in_bytes
out_file.write("bytes: %d\n" % in_bytes)
out_file.close()
