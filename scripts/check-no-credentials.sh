#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
python3 - <<'PY'
from contextlib import ExitStack
from pathlib import Path
import re
import shutil
import subprocess
import sys
from tempfile import TemporaryFile


def scan_blocks(stream):
    header = stream.read(25)
    # Only the simple lossless container emitted by our raster generator:
    # https://developers.google.com/speed/webp/docs/riff_container
    # Extended/animated WebP and all other formats retain the full byte scan.
    if header[:4] == b'RIFF' and header[8:16] == b'WEBPVP8L' and len(header) == 25:
        # Sparse Git blobs arrive through a pipe. Validate actual length before
        # excluding anything, using disk rather than buffering large images.
        with TemporaryFile() as image:
            image.write(header)
            shutil.copyfileobj(stream, image)
            length = image.tell()
            riff_end = int.from_bytes(header[4:8], 'little') + 8
            payload_size = int.from_bytes(header[16:20], 'little')
            pixel_end = 20 + payload_size
            valid = (payload_size > 5 and riff_end <= 0xfffffffe
                     and riff_end == pixel_end + payload_size % 2
                     and riff_end <= length and header[20] == 0x2f
                     and header[24] >> 5 == 0)
            if valid and payload_size % 2:
                image.seek(pixel_end)
                valid = image.read(1) == b'\0'
            image.seek(0)
            if valid:
                # Keep container/bitstream headers, padding and *all* trailing
                # bytes. A second chunk (including metadata) fails the single-
                # chunk layout check above and falls back to scanning everything.
                yield image.read(25)
                image.seek(pixel_end)
            yield from iter(lambda: image.read(65536), b'')
    else:
        yield header + stream.read(65536 - len(header))
        yield from iter(lambda: stream.read(65536), b'')

# Check bytes without decoding saves or echoing matches into logs/artifacts.
patterns = [
    re.compile(rb'PLAYER\t'),
    re.compile(rb'(?:^|[\r\n\x1b])(?:begin_save|end_save)(?=[\t\r\n\x1b]|$)'),
    re.compile(rb'''(?i)(?<![\w])["']?(?:password|passwd|pwd)["']?\s*[:=]'''),
]
paths = subprocess.check_output(['git', 'ls-files', '-t', '-z']).split(b'\0')[:-1]
failed = []
for raw in paths:
    sparse = raw[:2] == b'S '
    path = Path(raw[2:].decode())
    if path.is_symlink() or (not path.is_file() and not sparse):
        failed.append(str(path))
        continue
    with ExitStack() as stack:
        if sparse and not path.exists():
            # Sparse checkouts need no original artwork on disk. Still scan
            # indexed bytes; exclusion must never become a credential bypass.
            process = stack.enter_context(subprocess.Popen(
                ['git', 'cat-file', 'blob', ':' + str(path)], stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL))
            stream = process.stdout
        else:
            process = None
            stream = stack.enter_context(path.open('rb'))
        tail = b''
        for chunk in scan_blocks(stream):
            block = tail + chunk
            # Avoid running the full expressions over large image/PDF payloads.
            lower = block.lower()
            if any(token in lower for token in (b'player', b'begin_save', b'end_save', b'password', b'passwd', b'pwd')):
                if any(pattern.search(block) for pattern in patterns):
                    failed.append(str(path))
                    break
            tail = block[-1024:]
        if process is not None:
            stream.close()
            if process.wait() != 0 and str(path) not in failed:
                failed.append(str(path))
if failed:
    for path in failed:
        print(f'{path}: credential/save marker or unreadable tracked file (content withheld)', file=sys.stderr)
    sys.exit(1)
print(f'Credential check: {len(paths)} tracked files contain no credential/save markers outside validated compressed pixels; saves were not decoded.')
PY
