#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
python3 - <<'PY'
from pathlib import Path
import re
import subprocess
import sys

# Check bytes without decoding saves or echoing matches into logs/artifacts.
patterns = [
    re.compile(rb'PLAYER\t'),
    re.compile(rb'(?:^|[\r\n\x1b])(?:begin_save|end_save)(?=[\t\r\n\x1b]|$)'),
    re.compile(rb'''(?i)(?<![\w])["']?(?:password|passwd|pwd)["']?\s*[:=]'''),
]
paths = subprocess.check_output(['git', 'ls-files', '-z']).split(b'\0')[:-1]
failed = []
for raw in paths:
    path = Path(raw.decode())
    if path.is_symlink() or not path.is_file():
        failed.append(str(path))
        continue
    with path.open('rb') as stream:
        tail = b''
        while chunk := stream.read(65536):
            block = tail + chunk
            # Avoid running the full expressions over large image/PDF payloads.
            lower = block.lower()
            if any(token in lower for token in (b'player', b'begin_save', b'end_save', b'password', b'passwd', b'pwd')):
                if any(pattern.search(block) for pattern in patterns):
                    failed.append(str(path))
                    break
            tail = block[-1024:]
if failed:
    for path in failed:
        print(f'{path}: credential/save marker or unreadable tracked file (content withheld)', file=sys.stderr)
    sys.exit(1)
print(f'Credential check: {len(paths)} tracked files contain no credential/save markers; saves were not decoded.')
PY
