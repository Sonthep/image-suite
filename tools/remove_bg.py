#!/usr/bin/env python3
"""
CLI helper to remove background using rembg.
Usage: python tools/remove_bg.py input_path output_path
This writes a PNG with alpha at output_path.
"""
import sys
from rembg import remove, new_session
from PIL import Image

if __name__ == '__main__':
    if len(sys.argv) < 3:
        print('Usage: remove_bg.py <input> <output>')
        sys.exit(2)
    inp = sys.argv[1]
    outp = sys.argv[2]
    sess = new_session()
    with open(inp, 'rb') as f:
        data = f.read()
    res = remove(data, session=sess)
    with open(outp, 'wb') as f:
        f.write(res)
