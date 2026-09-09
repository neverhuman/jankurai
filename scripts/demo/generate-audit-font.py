#!/usr/bin/env python3
"""Rebuild the committed monochrome atlas from the exact documented font input."""
import hashlib
import json
from pathlib import Path
import sys
import PIL
from PIL import Image, ImageDraw, ImageFont

font_path, output = map(Path, sys.argv[1:3])
reference = json.loads(Path(__file__).with_name('audit-mono.json').read_text())
assert PIL.__version__ == '12.3.0'
assert hashlib.sha256(font_path.read_bytes()).hexdigest() == reference['source_sha256']
faces = {}
for scale in (1, 2, 4):
    width, height = 10 * scale, 20 * scale
    font = ImageFont.truetype(str(font_path), 16 * scale)
    glyphs = {}
    for number in range(32, 127):
        image = Image.new('1', (width, height))
        ImageDraw.Draw(image).text((0, 0), chr(number), font=font, fill=1)
        glyphs[chr(number)] = [sum(int(image.getpixel((x, y))) << x for x in range(width)) for y in range(height)]
    faces[str(scale)] = {'width': width, 'height': height, 'glyphs': glyphs}
reference.pop('glyphs', None)
reference['source'] = 'DejaVu Sans Mono, native 16/32/64px monochrome ASCII subsets'
reference['faces'] = faces
with output.open('x') as file:
    file.write(json.dumps(reference, separators=(',', ':')) + '\n')
