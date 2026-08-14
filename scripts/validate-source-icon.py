#!/usr/bin/env python3
"""Fail fast when a package-owned source icon cannot pass Koma's PNG boundary."""

import argparse
import binascii
import stat
import struct
import sys
import zlib
from pathlib import Path


MAX_BYTES = 1024 * 1024
MAX_DIMENSION = 1024
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


class ValidationError(Exception):
    pass


def fail(message):
    raise ValidationError(message)


def read_uint32(value, offset):
    return struct.unpack(">I", value[offset : offset + 4])[0]


def is_supported_color_layout(bit_depth, color_type):
    direct = color_type in (0, 2, 4, 6)
    indexed = color_type == 3
    return (
        (direct and bit_depth in (8, 16))
        or (indexed and bit_depth == 8)
    )


def channel_count(color_type):
    return {
        0: 1,
        2: 3,
        3: 1,
        4: 2,
        6: 4,
    }[color_type]


def validate_icon(path):
    try:
        path_stat = path.lstat()
    except OSError as error:
        fail("cannot read icon: {}".format(error))
    if stat.S_ISLNK(path_stat.st_mode):
        fail("icon.png must not be a symlink")
    if not stat.S_ISREG(path_stat.st_mode):
        fail("icon.png must be a regular file")
    if path_stat.st_size <= 0 or path_stat.st_size > MAX_BYTES:
        fail("icon.png must be between 1 byte and {} bytes".format(MAX_BYTES))

    try:
        value = path.read_bytes()
    except OSError as error:
        fail("cannot read icon bytes: {}".format(error))
    if len(value) < 57 or value[:8] != PNG_SIGNATURE:
        fail("icon.png must be a PNG")
    if value[8:12] != b"\x00\x00\x00\r" or value[12:16] != b"IHDR":
        fail("icon.png must start with IHDR")

    width = read_uint32(value, 16)
    height = read_uint32(value, 20)
    bit_depth = value[24]
    color_type = value[25]
    compression = value[26]
    filter_method = value[27]
    interlace = value[28]
    if not (0 < width <= MAX_DIMENSION and 0 < height <= MAX_DIMENSION):
        fail("icon.png dimensions must be within 1..{} px".format(MAX_DIMENSION))
    if not is_supported_color_layout(bit_depth, color_type):
        fail("icon.png must use an 8-bit indexed palette or 8-/16-bit direct PNG layout")
    if compression != 0 or filter_method != 0 or interlace != 0:
        fail("icon.png must use standard non-interlaced PNG encoding")

    offset = 8
    image_data = bytearray()
    saw_palette = False
    saw_image_data = False
    while offset + 12 <= len(value):
        length = read_uint32(value, offset)
        kind = value[offset + 4 : offset + 8]
        payload_offset = offset + 8
        next_offset = payload_offset + length + 4
        if next_offset > len(value):
            fail("icon.png contains a truncated PNG chunk")
        payload = value[payload_offset : payload_offset + length]
        stored_crc = read_uint32(value, payload_offset + length)
        if (binascii.crc32(kind + payload) & 0xFFFFFFFF) != stored_crc:
            fail("icon.png contains a PNG CRC mismatch")
        if offset == 8 and (kind != b"IHDR" or length != 13):
            fail("icon.png must begin with one IHDR chunk")
        if kind == b"PLTE":
            if saw_image_data or length == 0 or length % 3 != 0 or length > 768:
                fail("icon.png has an invalid palette chunk")
            saw_palette = True
        elif kind == b"IDAT":
            if color_type == 3 and not saw_palette:
                fail("indexed icon.png must include a palette before image data")
            if length > 0:
                image_data.extend(payload)
                saw_image_data = True
        elif kind == b"IEND":
            if length != 0 or not saw_image_data or next_offset != len(value):
                fail("icon.png must end with one terminal IEND chunk")
            row_bytes = (width * channel_count(color_type) * bit_depth + 7) // 8
            row_stride = row_bytes + 1
            expected_size = row_stride * height
            try:
                decoder = zlib.decompressobj()
                decoded = decoder.decompress(bytes(image_data), expected_size + 1)
            except zlib.error as error:
                fail("icon.png image data cannot be decoded: {}".format(error))
            if decoder.unconsumed_tail or decoder.unused_data or not decoder.eof:
                fail("icon.png image data has trailing or oversized compressed data")
            if len(decoded) > expected_size:
                fail("icon.png image data exceeds its declared dimensions")
            if len(decoded) != expected_size:
                fail("icon.png scanline data has an unexpected size")
            for row in range(height):
                if decoded[row * row_stride] > 4:
                    fail("icon.png uses an invalid scanline filter")
            return
        offset = next_offset
    fail("icon.png is missing a terminal IEND chunk")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--icon", required=True, help="path to an optional source icon.png")
    args = parser.parse_args()
    try:
        validate_icon(Path(args.icon))
    except ValidationError as error:
        print("source icon validation failed: {}".format(error), file=sys.stderr)
        return 1
    print("source icon validation passed: {}".format(args.icon))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
