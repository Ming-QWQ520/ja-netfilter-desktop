#!/usr/bin/env python3
"""Generate placeholder PNG icons for Tauri from a single orange square.

Tauri's bundler expects:
  - 32x32.png
  - 128x128.png
  - 128x128@2x.png (i.e. 256x256)
  - icon.icns (macOS)
  - icon.ico (Windows)

We generate the PNGs from scratch (no Pillow dependency) and assemble a
multi-frame .ico plus a PNG-based .icns that is sufficient for Tauri dev
builds and most release bundles. For distribution-grade .icns you may want
to regenerate from a 1024x1024 source via iconutil.
"""
import struct
import zlib
from pathlib import Path

OUT_DIR = Path(__file__).resolve().parent.parent / "download" / "ja-netfilter-desktop" / "src-tauri" / "icons"
OUT_DIR.mkdir(parents=True, exist_ok=True)


def make_png_rgba(size: int) -> bytes:
    """A solid orange square with a dark 2px border."""
    r, g, b, a = 254, 90, 61, 255
    rows = []
    for y in range(size):
        row = bytearray()
        for x in range(size):
            if x < 2 or x >= size - 2 or y < 2 or y >= size - 2:
                row += bytes((20, 22, 26, 255))
            else:
                row += bytes((r, g, b, a))
        rows.append(bytes(row))

    def chunk(tag: bytes, data: bytes) -> bytes:
        chunk_data = struct.pack(">I", len(data)) + tag + data
        crc = zlib.crc32(tag + data) & 0xFFFFFFFF
        return chunk_data + struct.pack(">I", crc)

    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    raw_with_filters = b""
    for row in rows:
        raw_with_filters += b"\x00" + row
    idat = zlib.compress(raw_with_filters, 9)
    return sig + chunk(b"IHDR", ihdr) + chunk(b"IDAT", idat) + chunk(b"IEND", b"")


def make_ico(sizes: list[int]) -> bytes:
    pngs = [(s, make_png_rgba(s)) for s in sizes]
    header = struct.pack("<HHH", 0, 1, len(pngs))
    directory = b""
    image_data = b""
    offset = 6 + 16 * len(pngs)
    for s, png in pngs:
        size_bytes = s if s < 256 else 0
        directory += struct.pack(
            "<BBBBHHII",
            size_bytes, size_bytes, 0, 0, 1, 32, len(png), offset
        )
        image_data += png
        offset += len(png)
    return header + directory + image_data


def main() -> None:
    for size, name in [(32, "32x32.png"), (128, "128x128.png"), (256, "128x128@2x.png")]:
        (OUT_DIR / name).write_bytes(make_png_rgba(size))
        print(f"wrote {name} ({size}x{size})")

    (OUT_DIR / "icon.png").write_bytes(make_png_rgba(512))
    print("wrote icon.png (512x512)")

    ico = make_ico([16, 32, 48, 64, 128, 256])
    (OUT_DIR / "icon.ico").write_bytes(ico)
    print(f"wrote icon.ico ({len(ico)} bytes)")

    png128 = make_png_rgba(128)
    png256 = make_png_rgba(256)
    png512 = make_png_rgba(512)
    png32 = make_png_rgba(32)

    def icns_chunk(tag: bytes, data: bytes) -> bytes:
        return tag + struct.pack(">I", len(data) + 8) + data

    body = (
        icns_chunk(b"ic07", png128)
        + icns_chunk(b"ic08", png256)
        + icns_chunk(b"ic09", png512)
        + icns_chunk(b"ic04", png32)
    )
    icns = b"icns" + struct.pack(">I", len(body) + 8) + body
    (OUT_DIR / "icon.icns").write_bytes(icns)
    print(f"wrote icon.icns ({len(icns)} bytes)")


if __name__ == "__main__":
    main()
