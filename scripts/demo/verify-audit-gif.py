#!/usr/bin/env python3
"""Independent Pillow decoder verifies the JS encoder's exact RGB frame hashes."""
import hashlib
import json
from pathlib import Path
import sys
import PIL
from PIL import Image

assert PIL.__version__ == '12.3.0', 'use the pinned independent decoder'
directory = Path(sys.argv[1])
manifest = json.loads((directory / 'audit-demo.json').read_text())
palette = {tuple(color) for color in manifest['palette']}
for output in manifest['outputs']:
    file = directory / output['name']
    raw = file.read_bytes()
    assert len(raw) == output['bytes'] < 50_000_000
    assert hashlib.sha256(raw).hexdigest() == output['sha256']
    with Image.open(file) as image:
        assert image.size == (output['width'], output['height'])
        assert image.n_frames == len(output['frames'])
        for index, expected in enumerate(output['frames']):
            image.seek(index)
            rgb = image.convert('RGB')
            assert hashlib.sha256(rgb.tobytes()).hexdigest() == expected['rgbSha256'], (file, index, 'pixels changed')
            assert image.info['duration'] == expected['durationMs']
            assert set(rgb.get_flattened_data()) <= palette, (file, index, 'unexpected dimmed/quantized color')
        image.convert('RGB').save(directory / (file.name + '.last.png'))
    print(f'{file.name}: {len(output["frames"])} exact RGB frames, {len(raw)} bytes, {output["width"]}x{output["height"]}')
