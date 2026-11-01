#!/usr/bin/env python3
"""Build content-addressed rule packs from independent JSON definitions."""
import argparse
import hashlib
import json
from pathlib import Path
from urllib.parse import urlparse
from PIL import Image

ROOT = Path(__file__).resolve().parent

def encode(obj):
    return (json.dumps(obj, ensure_ascii=False, separators=(',', ':')) + '\n').encode()

def require(condition, message):
    if not condition:
        raise ValueError(message)

def text(obj, key):
    value = obj.get(key)
    require(isinstance(value, str) and bool(value.strip()), f'{key} must be nonempty text')
    return value

def meta(obj):
    require(isinstance(obj, dict), 'meta must be an object')
    result = {key: text(obj, key) for key in ('name', 'description', 'website')}
    require(urlparse(result['website']).scheme == 'https' and bool(urlparse(result['website']).netloc), 'website must be HTTPS')
    return result

def signature(source, base):
    require(isinstance(source, dict), 'signature must be an object')
    name, filename = text(source, 'id'), text(source, 'image')
    with Image.open(base / filename) as image:
        image = image.convert('RGB')
        width, height = image.size
        require(width >= 16 and height > 0, 'invalid image dimensions')
        start, end = int(width * .2), int(width * .8)
        columns = [min(width - 1, start + int(i * ((end - start) / 16))) for i in range(16)]
        averages = []
        for y in range(height):
            row = [image.getpixel((x, y)) for x in columns]
            averages.extend(sum(pixel[c] for pixel in row) // 16 for c in range(3))
    result = {'id': name, 'width': width, 'height': height, 'averages': averages}
    if 'scale' in source:
        scale = source['scale']
        require(isinstance(scale, dict) and all(type(scale.get(k)) is int for k in ('minWidthExclusive', 'maxWidthExclusive', 'referenceWidth')), 'invalid scale')
        require(scale['minWidthExclusive'] < scale['maxWidthExclusive'] and scale['referenceWidth'] == width, 'invalid scale bounds/reference')
        result['scale'] = scale
    return result

def build_pack(path):
    definition = json.loads(path.read_text())
    require(isinstance(definition, dict), 'pack must be an object')
    pack_id = text(definition, 'id')
    version = definition.get('version')
    require(type(version) is int and version > 0, 'pack version must be positive integer')
    rules = definition.get('rules')
    require(isinstance(rules, list) and rules, 'pack requires rules')
    output, rule_ids = [], set()
    for rule in rules:
        require(isinstance(rule, dict), 'rule must be object')
        rule_id = text(rule, 'id')
        require(rule_id not in rule_ids, 'duplicate rule id')
        rule_ids.add(rule_id)
        require(text(rule, 'kind') == 'edge_signature_v1', 'unsupported rule kind')
        require(type(rule.get('crossSource')) is bool, 'crossSource must be boolean')
        signatures = rule.get('signatures')
        require(isinstance(signatures, list) and signatures, 'rule requires signatures')
        sigs = [signature(s, path.parent) for s in signatures]
        ids = {s['id'] for s in sigs}
        require(len(ids) == len(sigs), 'duplicate signature id')
        result = {'id': rule_id, 'name': text(rule, 'name'), 'description': text(rule, 'description'), 'kind': rule['kind'], 'crossSource': rule['crossSource'], 'signatures': sigs}
        for direction in ('top', 'bottom'):
            entries = rule.get(direction)
            require(isinstance(entries, list) and entries, f'{direction} requires entries')
            for entry in entries:
                require(isinstance(entry, dict) and entry.get('signature') in ids, f'invalid {direction} signature')
                if 'height' in entry:
                    original = next(s for s in sigs if s['id'] == entry['signature'])
                    require(type(entry['height']) is int and 0 < entry['height'] <= original['height'], 'invalid height')
            result[direction] = entries
        legacy = rule.get('legacy', [])
        require(isinstance(legacy, list), 'legacy must be an array')
        for entry in legacy:
            require(isinstance(entry, dict), 'legacy entry must be object')
            text(entry, 'kind')
            for k in ('singleModes', 'stackedModes', 'sourceIds'):
                if k in entry:
                    require(isinstance(entry[k], list) and all(isinstance(v, str) and v for v in entry[k]), f'invalid legacy {k}')
        result['legacy'] = legacy
        output.append(result)
    return {'schemaVersion': 1, 'id': pack_id, 'version': version, 'meta': meta(definition.get('meta')), 'rules': output}

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--definitions', type=Path, default=ROOT / 'definitions')
    parser.add_argument('--catalog', type=Path, default=ROOT / 'catalog.json')
    parser.add_argument('--output', type=Path, default=ROOT / 'dist')
    args = parser.parse_args()
    catalog = meta(json.loads(args.catalog.read_text()))
    files = sorted(args.definitions.glob('*.json'))
    require(bool(files), 'no pack definitions')
    packs, payloads, ids = [], [], set()
    for file in files:
        pack = build_pack(file)
        require(pack['id'] not in ids, 'duplicate pack id')
        ids.add(pack['id'])
        raw = encode(pack)
        digest = hashlib.sha256(raw).hexdigest()
        name = digest + '.json'
        packs.append({'id': pack['id'], 'version': pack['version'], 'path': name, 'sha256': digest})
        payloads.append((name, raw))
    # Revision changes for catalogue metadata or any pack identity/content change.
    revision = hashlib.sha256(encode({'meta': catalog, 'packs': packs})).hexdigest()
    index = {'schemaVersion': 1, 'kind': 'image_rules', 'revision': revision, 'meta': catalog, 'packs': packs}
    args.output.mkdir(parents=True, exist_ok=True)
    for name, raw in payloads:
        (args.output / name).write_bytes(raw)
    (args.output / 'index.json').write_bytes(encode(index))
    print(f"{len(packs)} packs, revision {revision}")

if __name__ == '__main__':
    main()
