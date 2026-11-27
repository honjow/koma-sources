#!/usr/bin/env python3
"""Execute the packaged shared viewer, including Kurage's verified ttb variant.

Usage: python3 sources/kuragebunch-ja/tests/test_viewer.py --package PATH.koma
Requires wasmtime and beautifulsoup4. The only HTTP response is a bounded
viewer fixture shaped from the 2026-11-28 Hatarakanai Futari episode. No network.
"""
import argparse
import copy
import html
import json
import struct
import zipfile

from bs4 import BeautifulSoup
import wasmtime

SERIES = '10834108156628842496'
EPISODE = '10834108156628843112'
ORIGIN = 'https://kuragebunch.com'
PAGE = 'https://cdn-img.kuragebunch.com/public/page/2/10834108156628847388-fixture'
BASE = {'readableProduct': {
    'id': EPISODE, 'typeName': 'episode', 'series': {'id': SERIES},
    'permalink': f'{ORIGIN}/episode/{EPISODE}', 'isPublic': True,
    'pageStructure': {'choJuGiga': 'baku', 'readingDirection': 'ttb', 'pages': [
        {'type': 'link'}, {'type': 'main', 'src': PAGE},
        {'type': 'main', 'src': PAGE + '2'}, {'type': 'link'},
    ]},
}}


def execute(engine, module, value):
    nodes = {}
    transfers = []
    store = wasmtime.Store(engine)

    def callback(name):
        def call(caller, *args):
            memory = caller.get('memory')

            def read(ptr, size):
                return bytes(memory.read(caller, ptr, ptr + size))

            def write(ptr, capacity, raw):
                assert len(raw) <= capacity
                memory.write(caller, raw, ptr)
                return len(raw)

            def add(node):
                handle = max(nodes, default=0) + 1
                nodes[handle] = node
                return handle

            if name == 'http_request':
                ptr, size, out, capacity = args
                request = json.loads(read(ptr, size))
                assert request['method'] == 'GET'
                assert request['url'] == f'{ORIGIN}/episode/{EPISODE}'
                transfers.append(request)
                document = '<script id="episode-json" data-value="' + html.escape(json.dumps(value), quote=True) + '"></script>'
                return write(out, capacity, json.dumps({'ok': True, 'status': 200, 'bodyText': document, 'networkPerformed': True}, separators=(',', ':')).encode())
            if name == 'html_parse':
                return add(BeautifulSoup(read(*args), 'html.parser'))
            if name == 'html_select':
                owner, ptr, size = args
                node = nodes[owner].select_one(read(ptr, size).decode())
                return add(node) if node is not None else 0
            if name == 'html_attr':
                owner, ptr, size, out, capacity = args
                return write(out, capacity, nodes[owner][read(ptr, size).decode()].encode())
            if name == 'html_close':
                del nodes[args[0]]
                return 0
            if name == 'check_cancel':
                return 0
            if name == 'log':
                return None
            raise AssertionError(f'Unexpected host operation: {name}')
        return call

    exports = wasmtime.Instance(store, module, [
        wasmtime.Func(store, item.type, callback(item.name), access_caller=True)
        for item in module.imports
    ]).exports(store)
    request = json.dumps({'type': 'request', 'version': 1, 'operation': 'get_pages', 'args': {
        'mangaId': f'kuragebunch:{SERIES}',
        'chapterId': f'kuragebunch-ch:{SERIES}:episode:{EPISODE}',
    }}).encode()
    memory = exports['memory']
    memory.write(store, request, 65536)
    ptr = exports['koma_source_get_pages'](store, 65536, len(request))
    magic, _, size, _ = struct.unpack('<IIII', memory.read(store, ptr, ptr + 16))
    assert magic == 0x4B4F4D41
    result = json.loads(bytes(memory.read(store, ptr + 16, ptr + 16 + size)))
    assert len(transfers) == 1 and not nodes
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--package', required=True)
    args = parser.parse_args()
    engine = wasmtime.Engine()
    with zipfile.ZipFile(args.package) as package:
        module = wasmtime.Module(engine, package.read('source.wasm'))
    for direction in ('ttb', 'rtl', 'ltr'):
        fixture = copy.deepcopy(BASE)
        fixture['readableProduct']['pageStructure']['readingDirection'] = direction
        result = execute(engine, module, fixture)
        assert result['ok'], result
        assert result['data']['readingDirection'] == direction
        assert [p['image']['url'] for p in result['data']['pages']] == [PAGE, PAGE + '2']
        assert [p['index'] for p in result['data']['pages']] == [0, 1]
    for change, expected in (
        ('unknown_direction', 'parse_error'), ('locked', 'chapter_locked'),
        ('wrong_series', 'parse_error'), ('unknown_scramble', 'unsupported_scramble'),
    ):
        fixture = copy.deepcopy(BASE)
        product = fixture['readableProduct']
        if change == 'unknown_direction':
            product['pageStructure']['readingDirection'] = 'diagonal'
        elif change == 'locked':
            product['isPublic'] = False
            product.pop('pageStructure')
        elif change == 'wrong_series':
            product['series']['id'] = '999'
        else:
            product['pageStructure']['choJuGiga'] = 'unknown'
        result = execute(engine, module, fixture)
        assert not result['ok'] and result['error']['code'] == expected, result
    print('PASS: ttb/rtl/ltr, page order, non-main exclusion, locked access and invalid viewer boundaries')


if __name__ == '__main__':
    main()
