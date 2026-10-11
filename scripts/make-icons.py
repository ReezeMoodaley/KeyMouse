"""Generate the repository's simple cursor icon without external image tools."""
import struct
import zlib
from pathlib import Path

def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))

def png(size):
    rows = bytearray()
    for y in range(size):
        rows.append(0)
        for x in range(size):
            u, v = x / size, y / size
            inside = .22 < u < .76 and .15 < v < .82 and u < .22 + (v - .15) * .94
            stem = .43 < u < .53 and .47 < v < .84
            rows.extend((183, 237, 158, 255) if inside or stem else (25, 29, 35, 255))
    header = struct.pack('>IIBBBBB', size, size, 8, 6, 0, 0, 0)
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', header) + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b'')

root = Path(__file__).resolve().parents[1] / 'src-tauri' / 'icons'
root.mkdir(parents=True, exist_ok=True)
for size in (32, 128):
    (root / f'{size}x{size}.png').write_bytes(png(size))
data = png(256)
(root / 'icon.ico').write_bytes(struct.pack('<HHH', 0, 1, 1) + struct.pack('<BBBBHHII', 0, 0, 0, 0, 1, 32, len(data), 22) + data)
