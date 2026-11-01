#!/usr/bin/env python3
"""Generate deterministic, content-addressed image rule resources."""
import hashlib
import json
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parent
OUT = ROOT / 'dist'
# Increment when the published pack changes.
PACK_VERSION = 2

def encode(value):
    return (json.dumps(value, ensure_ascii=False, separators=(',', ':')) + '\n').encode()

def signature(name, size):
    image = Image.open(ROOT / 'templates' / (name + '.jpg')).convert('RGB')
    if image.size != size:
        raise ValueError('unexpected template size: ' + name)
    width, height = size
    start, end = int(width * .2), int(width * .8)
    columns = [min(width - 1, start + int(i * ((end - start) / 16))) for i in range(16)]
    averages = []
    for y in range(height):
        row = [image.getpixel((x, y)) for x in columns]
        averages.extend(sum(pixel[c] for pixel in row) // 16 for c in range(3))
    result = {'id': name, 'width': width, 'height': height, 'averages': averages}
    if name == 'narrow':
        result['scale'] = {'minWidthExclusive': 640, 'maxWidthExclusive': 800, 'referenceWidth': 640}
    return result

rule = {'id': 'edge-banners', 'kind': 'edge_signature_v1', 'crossSource': True,
        'signatures': [signature('current', (800, 200)), signature('old', (800, 282)), signature('narrow', (640, 186))],
        'top': [{'signature': n} for n in ('current', 'old', 'narrow')],
        'bottom': [{'signature': 'old'}, {'signature': 'current'}, {'signature': 'narrow', 'height': 160}],
        'legacy': [{'kind': 'baozi_banner', 'singleModes': ['1'], 'stackedModes': ['2'], 'sourceIds': ['online.baozimh.koma']}]}
pack = encode({'schemaVersion': 1, 'id': 'edge-banners', 'version': PACK_VERSION, 'rules': [rule]})
digest = hashlib.sha256(pack).hexdigest()
path = digest + '.json'
index = encode({'schemaVersion': 1, 'revision': digest, 'packs': [{'id': 'edge-banners', 'version': PACK_VERSION, 'path': path, 'sha256': digest}]})
(OUT / 'packs').mkdir(parents=True, exist_ok=True)
(OUT / path).write_bytes(pack)
# Keep the previous branch layout available to clients holding an old index.
(OUT / 'packs' / path).write_bytes(pack)
(OUT / 'index.json').write_bytes(index)
print(OUT / path, digest)
