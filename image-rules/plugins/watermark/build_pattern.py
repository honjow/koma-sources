#!/usr/bin/env python3
"""Derive a grayscale signature from verified, identically placed watermark samples."""
import argparse
import struct
from pathlib import Path
from statistics import median

from PIL import Image


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("samples", nargs="+", type=Path)
    parser.add_argument("--page-width", type=int, default=800)
    parser.add_argument("--box", nargs=4, type=int, required=True,
                        metavar=("RIGHT", "BOTTOM", "WIDTH", "HEIGHT"),
                        help="bottom-right insets and rectangle size in original pixels")
    parser.add_argument("--background", choices=("white", "black"), default="white")
    args = parser.parse_args()
    right, bottom, width, height = args.box
    background = 255 if args.background == "white" else 1
    if not (0 <= right < args.page_width <= 65535 and
            0 <= bottom <= 65535 and 0 < width <= args.page_width - right and 0 < height <= 65535):
        parser.error("invalid rectangle")

    rows = []
    for sample in args.samples:
        with Image.open(sample) as image:
            w, h = image.size
            if w != args.page_width or h < height + bottom:
                parser.error(f"incompatible sample size: {sample}")
            box = (w - right - width, h - bottom - height, w - right, h - bottom)
            rows.append(image.crop(box).convert("L").tobytes())

    pixels = bytes(round(median(values)) for values in zip(*rows))
    perimeter = pixels[:width] + pixels[-width:] + pixels[::width] + pixels[width - 1::width]
    if any(abs(pixel - background) > 12 for pixel in perimeter):
        parser.error("signature edge contains content. Choose aligned samples with a clean border.")
    if sum(abs(pixel - background) >= 14 for pixel in pixels) < 64:
        parser.error("signature has too little contrast")
    args.output.write_bytes(struct.pack("<5HB", args.page_width, right, bottom, width, height, background) + pixels)


if __name__ == "__main__":
    main()
