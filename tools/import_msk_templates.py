"""Reproduce the vendored Apache-2.0 MSK catalogue; never executes upstream TS.

Explicit developer command only. The calculator itself does not use the network.
"""
import hashlib
import json
import re
from pathlib import Path
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parent.parent
REVISION = '946049d80f0f4b4956a53383961c8c6f4431169f'
BASE = f'https://raw.githubusercontent.com/geo-ts/msk/{REVISION}/'
HASHES = {
    'index.ts': '4862fc4121c2afd0919b7a0349dbc2cfb0264a82a8e4263da7610b28b59b4cad',
    'LICENSE': '43070e2d4e532684de521b885f385d0841030efa2b1a20bafb76133a5e1379c1',
}


def download(name):
    with urlopen(BASE + name, timeout=45) as response:
        data = response.read()
    digest = hashlib.sha256(data).hexdigest()
    if digest != HASHES[name]:
        raise ValueError(f'Unexpected upstream bytes: {name}')
    print(name, 'SHA256', digest)
    return data.decode('utf-8')


def main():
    source = download('index.ts')
    start = source.index('= [', source.index('export let msk')) + 2
    records = json.loads(source[start:source.index('\n]', start) + 2])
    assert len(records) == 262 and len({r['id'] for r in records}) == 262
    lines = [
        '# SPDX-License-Identifier: Apache-2.0',
        '# Adapted from geo-ts/msk index.ts at ' + REVISION,
        '# https://github.com/geo-ts/msk; see third_party/geo-ts-msk-LICENSE.txt',
        '# Changes: TSV serialization, stable region keys, whitespace normalization;',
        '# Krasnoyarsk city/local systems grouped under region 24. Numbers unchanged.',
        '# id\tregion\tupstream name\tPROJ definition',
    ]
    for record in records:
        ident = record['id']
        if record['region'] in ('Красноярский Край', 'г. Красноярск'):
            region = '24'
        elif ident == 'MSKMGGT':
            region = '77'
        elif ident == 'MSK1964sp':
            region = '78'
        else:
            region = re.fullmatch(r'MSK(\d{2}).*', ident).group(1)
        # One upstream towgs84 value contains spaces after commas. Preserve all
        # numeric values while making each PROJ parameter a single token.
        definition = re.sub(r'\s*,\s*', ',', record['proj'].strip())
        row = [ident, region, record['name'].strip(), definition]
        assert all(not any(c in cell for c in '\t\r\n') for cell in row)
        lines.append('\t'.join(row))
    data_dir = ROOT / 'data'
    license_dir = ROOT / 'third_party'
    data_dir.mkdir(exist_ok=True)
    license_dir.mkdir(exist_ok=True)
    (data_dir / 'msk_templates.tsv').write_text('\n'.join(lines) + '\n', encoding='utf-8', newline='\n')
    (license_dir / 'geo-ts-msk-LICENSE.txt').write_text(download('LICENSE'), encoding='utf-8', newline='\n')
    print(f'Imported {len(records)} templates / {len(set(row.split(chr(9))[1] for row in lines[6:]))} regions.')


if __name__ == '__main__':
    main()
