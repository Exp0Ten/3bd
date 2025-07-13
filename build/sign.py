#!/bin/python3

import sys
import os

def getSize(filename):
    return os.stat(filename).st_size

size = getSize(sys.argv[1])

os.write(1, size.to_bytes(4, 'little'))