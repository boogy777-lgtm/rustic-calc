"""Small, dependency-free checks for the documented project/module constraints."""
import argparse
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--tag')
    args = parser.parse_args()
    errors = []
    cargo = tomllib.loads((ROOT / 'Cargo.toml').read_text(encoding='utf-8'))
    package = cargo['package']
    if package['license'] != 'MIT':
        errors.append('License must remain MIT unless explicitly approved.')
    if set(cargo.get('dependencies', {})) != {'windows-sys'}:
        errors.append('New runtime dependencies require approval; baseline is only windows-sys.')
    if cargo['dependencies']['windows-sys']['version'] != '0.59':
        errors.append('Review and approve any windows-sys SDK upgrade explicitly.')
    config = (ROOT / '.cargo/config.toml').read_text(encoding='utf-8')
    if 'target-cpu=native' in config:
        errors.append('Distributed builds cannot use target-cpu=native.')
    if args.tag and args.tag != 'v' + package['version']:
        errors.append('Release tag and Cargo version differ.')
    for name in ['README.md', 'LICENSE', 'AGENTS.md', 'CONTRIBUTING.md', 'SECURITY.md',
                 'docs/ARCHITECTURE.md', 'docs/MODULE_SDK.md', 'docs/LLM_RULES.md',
                 'docs/RELEASING.md']:
        if not (ROOT / name).is_file():
            errors.append(f'Missing project contract: {name}')
    engine = (ROOT / 'src/engine.rs').read_text(encoding='utf-8')
    if re.search(r'windows_sys|crate::platform|std::fs|std::process', engine):
        errors.append('Numerical engine must not depend on Win32/platform/I/O.')
    for path in (ROOT / 'src/views').glob('*.rs'):
        production = path.read_text(encoding='utf-8').split('#[cfg(test)]')[0]
        if re.search(r'\bunsafe\s*(?:\{|fn\b)', production):
            errors.append(f'Unsafe production view code: {path.name}')
        if re.search(r'Lang::(?:Ru|En)\s*=>\s*"', production):
            errors.append(f'Inline localized text belongs in i18n: {path.name}')
    for path in (ROOT / 'src').rglob('*.rs'):
        text = path.read_text(encoding='utf-8')
        if re.search(r'C:\\Users\\', text, re.IGNORECASE):
            errors.append(f'Private machine path in {path.relative_to(ROOT)}')
    if errors:
        print('\n'.join('ERROR: ' + message for message in errors), file=sys.stderr)
        return 1
    print(f'Project/module contracts OK: Rustic Calc {package["version"]}')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
