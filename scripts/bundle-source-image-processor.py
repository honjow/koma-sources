#!/usr/bin/env python3
"""Embed a standalone processor in an ordinary, portable source.wasm."""

from hashlib import sha256
from pathlib import Path
import sys


WASM_HEADER = b"\0asm\x01\0\0\0"
SECTION_NAME = b"koma.image-processor.v1"


def leb128(value):
    encoded = bytearray()
    while value >= 128:
        encoded.append((value & 127) | 128)
        value >>= 7
    encoded.append(value)
    return bytes(encoded)


def read_length(data, offset, end):
    length = 0
    for shift in range(0, 32, 7):
        if offset >= end or (shift == 28 and data[offset] > 15):
            break
        byte = data[offset]
        offset += 1
        length |= (byte & 127) << shift
        if byte < 128:
            return length, offset
    raise ValueError("invalid WASM section length")


def bundle(source, processor):
    if not source.startswith(WASM_HEADER) or not processor.startswith(WASM_HEADER):
        raise ValueError("expected source and processor WASM modules")
    if len(processor) > 2 * 1024 * 1024:
        raise ValueError("image processor exceeds 2 MiB")
    cursor = len(WASM_HEADER)
    while cursor < len(source):
        kind = source[cursor]
        size, start = read_length(source, cursor + 1, len(source))
        cursor = start + size
        if cursor > len(source):
            raise ValueError("incomplete source WASM section")
        if kind == 0:
            name_size, name_start = read_length(source, start, cursor)
            name_end = name_start + name_size
            if name_end > cursor:
                raise ValueError("incomplete source WASM section name")
            if source[name_start:name_end] == SECTION_NAME:
                raise ValueError("source already contains an image processor")

    payload = leb128(len(SECTION_NAME)) + SECTION_NAME + processor
    return source + b"\0" + leb128(len(payload)) + payload


if __name__ == "__main__":
    if len(sys.argv) != 4:
        raise SystemExit("usage: bundle-source-image-processor.py SOURCE PROCESSOR OUTPUT")
    source_path, processor_path, output_path = map(Path, sys.argv[1:])
    try:
        wasm = bundle(source_path.read_bytes(), processor_path.read_bytes())
        output_path.write_bytes(wasm)
    except (OSError, ValueError) as error:
        raise SystemExit(str(error)) from error
    print(sha256(wasm).hexdigest())
