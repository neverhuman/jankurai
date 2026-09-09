#!/usr/bin/env python3
"""Synthetic decoder controls; these fixtures are never demo audit recordings."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from PIL import Image

VERIFIER = Path(__file__).with_name('verify-audit-gif.py')


class DecoderControls(unittest.TestCase):
    def verify_case(self, change=None, pixels=None, expected_error=None, read_only=False):
        with tempfile.TemporaryDirectory(prefix='jankurai-decoder-control-') as temporary:
            directory = Path(temporary)
            palette = [[index * 15, 40, 210] for index in range(16)]
            colors = pixels or [tuple(palette[0]), tuple(palette[1])]
            image = Image.new('RGB', (len(colors), 1))
            image.putdata(colors)
            file = directory / 'synthetic-control.gif'
            image.save(file, format='GIF', duration=100, optimize=False)
            raw = file.read_bytes()
            with Image.open(file) as decoded:
                rgb = decoded.convert('RGB').tobytes()
            output = {
                'name': file.name, 'bytes': len(raw),
                'sha256': hashlib.sha256(raw).hexdigest(),
                'width': len(colors), 'height': 1,
                'frames': [{'rgbSha256': hashlib.sha256(rgb).hexdigest(), 'durationMs': 100}],
            }
            if change:
                change(output)
            (directory / 'audit-demo.json').write_text(json.dumps({'palette': palette, 'outputs': [output]}))
            result = subprocess.run([sys.executable, str(VERIFIER), str(directory)] + (['--read-only'] if read_only else []), capture_output=True, text=True, timeout=10)
            if expected_error is None:
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual((directory / (file.name + '.last.png')).is_file(), not read_only)
            else:
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(expected_error, result.stderr)

    def test_matching_pixels_timing_palette_and_digests_pass(self):
        self.verify_case()

    def test_read_only_check_does_not_write_preview(self):
        self.verify_case(read_only=True)

    def test_read_only_still_rejects_wrong_pixels(self):
        self.verify_case(lambda item: item['frames'][0].update(rgbSha256='0' * 64), expected_error='pixels changed', read_only=True)

    def test_changed_pixel_digest_fails(self):
        self.verify_case(lambda item: item['frames'][0].update(rgbSha256='0' * 64), expected_error='pixels changed')

    def test_changed_duration_fails(self):
        self.verify_case(lambda item: item['frames'][0].update(durationMs=110), expected_error='AssertionError')

    def test_changed_file_digest_fails(self):
        self.verify_case(lambda item: item.update(sha256='0' * 64), expected_error='AssertionError')

    def test_unknown_color_fails_even_when_all_digests_match(self):
        self.verify_case(pixels=[(7, 8, 9), (0, 40, 210)], expected_error='unexpected dimmed/quantized color')

    def test_color_overflow_fails_even_when_all_digests_match(self):
        self.verify_case(pixels=[(index * 15, 40, 210) for index in range(16)] + [(7, 8, 9)], expected_error='unexpected dimmed/quantized color')


if __name__ == '__main__':
    unittest.main()
