"""Reject portable Windows executables that need a separate MSVC runtime."""

import argparse
import re
import struct
from pathlib import Path


MSVC_RUNTIME = re.compile(rb'(?:vcruntime|msvcp|concrt)14[0-9][a-z0-9_]*\.dll', re.I)


def imports(data):
    def unpack(fmt, offset):
        if offset < 0:
            raise ValueError('negative PE offset')
        return struct.unpack_from('<' + fmt, data, offset)

    if data[:2] != b'MZ':
        raise ValueError('not a DOS/PE image')
    pe, = unpack('I', 0x3c)
    if data[pe:pe + 4] != b'PE\0\0':
        raise ValueError('missing PE signature')
    _, section_count, _, _, _, optional_size, _ = unpack('HHIIIHH', pe + 4)
    opt = pe + 24
    if optional_size < 112 or unpack('H', opt)[0] != 0x20b:
        raise ValueError('expected a PE32+ optional header')
    directory_count, = unpack('I', opt + 108)
    if directory_count > (optional_size - 112) // 8:
        raise ValueError('PE data directories extend beyond the optional header')
    sections = [
        unpack('IIII', opt + optional_size + index * 40 + 8)
        for index in range(section_count)
    ]

    def locate(rva, minimum_size):
        for _, start, raw_size, raw in sections:
            if start <= rva and rva - start + minimum_size <= raw_size:
                offset = raw + rva - start
                end = min(raw + raw_size, len(data))
                if offset + minimum_size <= end:
                    return offset, end
        raise ValueError(f'invalid PE RVA {rva:#x}')

    for index, size, name_slot in [(1, 20, 3), (13, 32, 1)]:
        if index >= directory_count:
            continue
        rva, directory_size = unpack('II', opt + 112 + index * 8)
        if not rva and not directory_size:
            continue
        if not rva or directory_size < size:
            raise ValueError('invalid PE import directory')
        start, end = locate(rva, directory_size)
        for offset in range(start, min(start + directory_size, end) - size + 1, size):
            descriptor = unpack('I' * (size // 4), offset)
            if not any(descriptor):
                break
            name_rva = descriptor[name_slot]
            if index == 13 and not descriptor[0] & 1:
                raise ValueError('unsupported PE delay-import address format')
            name_offset, end_of_section = locate(name_rva, 1)
            name_end = data.find(b'\0', name_offset, end_of_section)
            if name_end < 0:
                raise ValueError('unterminated PE DLL name')
            yield data[name_offset:name_end]
        else:
            raise ValueError('unterminated PE import directory')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('exe', type=Path)
    args = parser.parse_args()
    try:
        blocked = sorted({name.decode('ascii') for name in imports(args.exe.read_bytes())
                          if MSVC_RUNTIME.fullmatch(name)})
    except (OSError, ValueError, struct.error) as error:
        parser.exit(1, f'Cannot verify Windows imports: {error}\n')
    if blocked:
        parser.exit(1, f'Separate MSVC runtime imported: {", ".join(blocked)}. Use Build-Portable.ps1.\n')


if __name__ == '__main__':
    main()
