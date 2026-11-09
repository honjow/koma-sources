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
                        metavar=("EDGE", "BOTTOM", "WIDTH", "HEIGHT"),
                        help="side and bottom insets plus rectangle size in original pixels")
    parser.add_argument("--side", choices=("left", "right"), default="right")
    parser.add_argument("--background", choices=("white", "black"), default="white")
    args = parser.parse_args()
    inset, bottom, width, height = args.box
    background = 255 if args.background == "white" else 1
    if not (0 <= inset < args.page_width <= 65535 and
            0 <= bottom <= 65535 and 0 < width <= args.page_width - inset and 0 < height <= 65535):
        parser.error("invalid rectangle")

    rows = []
    for sample in args.samples:
        with Image.open(sample) as image:
            w, h = image.size
            if w != args.page_width or h < height + bottom:
                parser.error(f"incompatible sample size: {sample}")
            left = inset if args.side == "left" else w - inset - width
            crop = image.crop((left, h - bottom - height, left + width, h - bottom)).convert("L")
            if args.side == "left":
                # Store both sides in the same edge-relative coordinate system.
                crop = crop.transpose(Image.Transpose.FLIP_LEFT_RIGHT)
            rows.append(crop.tobytes())

    pixels = bytes(round(median(values)) for values in zip(*rows))
    perimeter = pixels[:width] + pixels[-width:] + pixels[::width] + pixels[width - 1::width]
    if any(abs(pixel - background) > 12 for pixel in perimeter):
        parser.error("signature edge contains content. Choose aligned samples with a clean border.")
    if sum(abs(pixel - background) >= 14 for pixel in pixels) < 64:
        parser.error("signature has too little contrast")
    args.output.write_bytes(struct.pack("<5HB", args.page_width, inset, bottom, width, height, background) + pixels)


if __name__ == "__main__":
    main()
