#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
python3 - <<'PY'
from contextlib import ExitStack
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
        while chunk := stream.read(65536):
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
print(f'Credential check: {len(paths)} tracked files contain no credential/save markers; saves were not decoded.')
PY
