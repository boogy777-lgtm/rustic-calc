import struct
import unittest

from check_windows_imports import MSVC_RUNTIME, imports


def image(dll, delay=False):
    data = bytearray(0x300)
    pe = 0x80
    opt = pe + 24
    data[:2] = b'MZ'
    struct.pack_into('<I', data, 0x3c, pe)
    data[pe:pe + 4] = b'PE\0\0'
    struct.pack_into('<HHIIIHH', data, pe + 4, 0x8664, 1, 0, 0, 0, 240, 2)
    struct.pack_into('<H', data, opt, 0x20b)
    struct.pack_into('<I', data, opt + 108, 16)
    struct.pack_into('<IIII', data, opt + 240 + 8, 0x100, 0x1000, 0x100, 0x200)
    if delay:
        struct.pack_into('<II', data, opt + 112 + 13 * 8, 0x1000, 64)
        struct.pack_into('<II', data, 0x200, 1, 0x1050)
    else:
        struct.pack_into('<II', data, opt + 112 + 8, 0x1000, 40)
        struct.pack_into('<I', data, 0x20c, 0x1050)
    data[0x250:0x250 + len(dll)] = dll
    return data


class PortableImportsTest(unittest.TestCase):
    def test_system_import_allowed(self):
        self.assertEqual(list(imports(image(b'KERNEL32.dll'))), [b'KERNEL32.dll'])
        self.assertIsNone(MSVC_RUNTIME.fullmatch(b'KERNEL32.dll'))

    def test_runtime_detected_in_both_tables(self):
        for delay in (False, True):
            dll, = imports(image(b'VCRUNTIME140_1.dll', delay))
            self.assertIsNotNone(MSVC_RUNTIME.fullmatch(dll))

    def test_invalid_import_offset_is_rejected(self):
        data = image(b'kernel32.dll')
        struct.pack_into('<I', data, 0x20c, 0x2000)
        with self.assertRaisesRegex(ValueError, 'invalid PE RVA'):
            list(imports(data))


if __name__ == '__main__':
    unittest.main()
