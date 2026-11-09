"""Windows foreground UI smoke test against a test-owned Rustic Calc process.

Uses control/window messages, not global keyboard injection or the clipboard.
Pillow is optional for filled-form screenshots; not an application dependency.
"""
import argparse
import ctypes as c
from ctypes import wintypes as w
from pathlib import Path
import subprocess
import time

u = c.WinDLL('user32', use_last_error=True)
u.SendMessageW.argtypes = [w.HWND, w.UINT, w.WPARAM, w.LPARAM]
u.SendMessageW.restype = w.LPARAM
u.PostMessageW.argtypes = [w.HWND, w.UINT, w.WPARAM, w.LPARAM]
u.PostMessageW.restype = w.BOOL
u.GetDlgItem.argtypes = [w.HWND, c.c_int]
u.GetDlgItem.restype = w.HWND
u.GetWindowThreadProcessId.argtypes = [w.HWND, c.POINTER(w.DWORD)]
u.GetWindowThreadProcessId.restype = w.DWORD
u.WaitForInputIdle.argtypes = [w.HANDLE, w.DWORD]
u.WaitForInputIdle.restype = w.DWORD
u.IsWindowVisible.argtypes = [w.HWND]
u.IsWindowVisible.restype = w.BOOL
u.EnumWindows.argtypes = [c.WINFUNCTYPE(w.BOOL, w.HWND, w.LPARAM), w.LPARAM]
u.EnumWindows.restype = w.BOOL


class GuiThreadInfo(c.Structure):
    _fields_ = [('cbSize', w.DWORD), ('flags', w.DWORD),
                ('active', w.HWND), ('focus', w.HWND), ('capture', w.HWND),
                ('menuOwner', w.HWND), ('moveSize', w.HWND), ('caret', w.HWND),
                ('caretRect', w.RECT)]


u.GetGUIThreadInfo.argtypes = [w.DWORD, c.POINTER(GuiThreadInfo)]
u.GetGUIThreadInfo.restype = w.BOOL


def wait_for(predicate, message):
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(0.02)
    raise AssertionError(message)


def find_window(pid):
    found = []

    @c.WINFUNCTYPE(w.BOOL, w.HWND, w.LPARAM)
    def visit(hwnd, _):
        owner = w.DWORD()
        u.GetWindowThreadProcessId(hwnd, c.byref(owner))
        if owner.value == pid and u.IsWindowVisible(hwnd):
            found.append(hwnd)
        return True

    u.EnumWindows(visit, 0)
    assert len(found) == 1, found
    return found[0]


def read_text(hwnd):
    size = u.SendMessageW(hwnd, 0x000E, 0, 0)  # WM_GETTEXTLENGTH (cross-process)
    buffer = c.create_unicode_buffer(size + 1)
    u.SendMessageW(hwnd, 0x000D, len(buffer), c.addressof(buffer))
    return buffer.value


def write_text(hwnd, value):
    buffer = c.create_unicode_buffer(value)
    # WM_SETTEXT does NOT issue EN_CHANGE for a multiline EDIT. Replace the
    # selection like real typing/paste so the test exercises input→state wiring.
    u.SendMessageW(hwnd, 0x00B1, 0, -1)  # EM_SETSEL
    u.SendMessageW(hwnd, 0x00C2, 1, c.addressof(buffer))  # EM_REPLACESEL


def run(binary, lang, theme, output):
    process = subprocess.Popen([str(binary), '--mode', 'coordinates', '--lang', lang, '--theme', theme])
    hwnd = None
    try:
        assert u.WaitForInputIdle(int(process._handle), 15000) == 0
        hwnd = find_window(process.pid)
        child = lambda ident: u.GetDlgItem(hwnd, ident)
        assert u.SendMessageW(child(500), 0x0146, 0, 0) == 89  # CB_GETCOUNT

        def choose(ident, index):
            u.SendMessageW(child(ident), 0x014E, index, 0)  # CB_SETCURSEL
            u.SendMessageW(hwnd, 0x0111, ident | (1 << 16), child(ident))

        choose(501, 2)  # MSK-50 zone 2, following the Custom row.
        definition = read_text(child(502))
        assert '+lon_0=38.48333333333' in definition
        for ident, value in [(503, '55,7558'), (504, '37.6173'), (505, '150')]:
            write_text(child(ident), value)

        def calculate():
            # Queued key to a real native field exercises the actual message
            # loop's Enter forwarding, not just an App::action unit test.
            assert u.PostMessageW(child(503), 0x0100, 0x0D, 0)
            wait_for(lambda: bool(read_text(child(506))), 'native Enter did not calculate')
            return float(read_text(child(506))), float(read_text(child(507)))

        northing, easting = calculate()
        thread = u.GetWindowThreadProcessId(hwnd, None)
        assert u.PostMessageW(child(503), 0x0100, 0x09, 0)  # native Tab

        def longitude_has_focus():
            info = GuiThreadInfo()
            info.cbSize = c.sizeof(info)
            assert u.GetGUIThreadInfo(thread, c.byref(info))
            return info.focus == child(504)

        wait_for(longitude_has_focus, 'Tab did not focus longitude')

        modified = definition.replace('+x_0=2250000', '+x_0=2251000')
        write_text(child(502), modified)
        assert read_text(child(506)) == '' and read_text(child(507)) == ''
        n2, e2 = calculate()
        assert abs(n2 - northing) < 0.001 and abs(e2 - easting - 1000) < 0.001

        # A real EDIT replacement updates state and clears stale outputs.
        u.SendMessageW(child(503), 0x00B1, 0, -1)  # EM_SETSEL
        u.SendMessageW(child(503), 0x0102, ord('5'), 0)  # WM_CHAR
        assert read_text(child(503)) == '5' and read_text(child(506)) == ''
        write_text(child(503), '55,7558')
        write_text(child(505), '')
        u.SendMessageW(hwnd, 0x0100, 0x0D, 0)
        assert read_text(child(506)) == ''  # blank height never silently becomes 0
        write_text(child(505), '150')
        calculate()

        # Missing-template region: no parameters are fabricated. Return to the
        # modified template to check both selection and text preservation.
        choose(500, 16)  # Tuva
        assert read_text(child(502)) == ''
        assert u.SendMessageW(child(501), 0x0146, 0, 0) == 1
        choose(500, 49)  # Moscow Oblast
        assert read_text(child(502)) == modified

        # Switching away and back must retain uncalculated edits.
        u.SendMessageW(hwnd, 0x0111, 1001, 0)
        u.SendMessageW(hwnd, 0x0111, 1007, 0)
        assert read_text(child(502)) == modified
        assert read_text(child(503)) == '55,7558'
        u.SendMessageW(hwnd, 0x0111, 1111 if lang == 'ru' else 1110, 0)
        assert read_text(child(502)) == modified
        u.SendMessageW(hwnd, 0x0111, 1110 if lang == 'ru' else 1111, 0)
        u.SendMessageW(hwnd, 0x0111, 1122 if theme == 'white' else 1121, 0)
        assert read_text(child(502)) == modified
        u.SendMessageW(hwnd, 0x0111, 1121 if theme == 'white' else 1122, 0)

        assert u.PostMessageW(child(503), 0x0100, 0x71, 0)  # F2 from EDIT
        wait_for(lambda: bool(u.IsWindowVisible(child(508))), 'F2 did not open help')
        help_text = read_text(child(508))
        assert '946049d80f0f4b4956a53383961c8c6f4431169f' in help_text
        assert 'END OF TERMS AND CONDITIONS' in help_text
        assert help_text.rstrip().endswith('limitations under the License.')
        assert u.PostMessageW(child(508), 0x0100, 0x1B, 0)  # Esc from help
        wait_for(lambda: not u.IsWindowVisible(child(508)), 'Esc did not close help')
        assert read_text(child(502)) == modified
        calculate()
        try:
            from PIL import ImageGrab
            ImageGrab.grab(window=hwnd).save(output / f'{theme}-coordinates-{lang}-filled.png')
        except ImportError:
            print('Pillow unavailable; filled-form screenshot skipped.')
        print(f'PASS {lang}/{theme}: live fields, Enter, Tab, dirty results, custom PROJ, region/mode/locale/theme drafts, F2/Esc/license.')
    finally:
        if hwnd:
            u.PostMessageW(hwnd, 0x0010, 0, 0)  # WM_CLOSE only our test process
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.terminate()  # Only the process started by this test.
            process.wait(timeout=5)
        assert process.returncode == 0, f'Test process exit {process.returncode}'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('binary', type=Path)
    parser.add_argument('--output', type=Path, default=Path('artifacts/screenshots'))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    for theme in ['white', 'dark']:
        for lang in ['ru', 'en']:
            run(args.binary.resolve(), lang, theme, args.output)


if __name__ == '__main__':
    main()
