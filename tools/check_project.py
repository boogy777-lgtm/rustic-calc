"""Small, dependency-free checks for the documented project/module constraints."""
import argparse
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
APP = Path('crates/calc-app')
EDITION_DEPS = {
    'slint': ({'slint': '=1.18.1', 'windows-sys': '0.59'}, {'slint-build': '=1.18.1'}),
    'gpui': ({'gpui': '=0.2.2', 'windows-sys': '0.59'}, {}),
}


def check_edition(name, version, errors):
    path = ROOT / 'editions' / name / 'Cargo.toml'
    if not path.is_file():
        return
    cargo = tomllib.loads(path.read_text(encoding='utf-8'))
    if cargo['package']['version'] != version or cargo['package'].get('publish') is not False:
        errors.append(f'{name}: edition must match the release version and remain unpublished.')
    deps = cargo.get('dependencies', {})
    builds = cargo.get('build-dependencies', {})
    approved, approved_build = EDITION_DEPS[name]
    needed = {'rustic-calc-app', name}
    if not needed <= deps.keys() or not deps.keys() <= needed | {'windows-sys'}:
        errors.append(f'{name}: unapproved runtime dependency set.')
    if deps.get('rustic-calc-app') != {'path': '../../crates/calc-app'}:
        errors.append(f'{name}: the application layer must use only the local crate.')
    for table, rules in ((deps, approved), (builds, approved_build)):
        for dep, spec in table.items():
            if dep == 'rustic-calc-app' and table is deps:
                continue
            requested = spec.get('version') if isinstance(spec, dict) else spec
            if dep not in rules or rules[dep] != requested or (
                isinstance(spec, dict) and set(spec) - {'version', 'features', 'default-features'}
            ):
                errors.append(f'{name}: dependencies must use reviewed registry versions ({dep}).')
    if set(builds) != set(approved_build):
        errors.append(f'{name}: build dependency set does not match the reviewed tooling.')
    if any(cargo.get(key) for key in ('dev-dependencies', 'target', 'patch', 'replace')):
        errors.append(f'{name}: additional target or patched dependencies require approval.')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--tag')
    args = parser.parse_args()
    errors = []
    cargo = tomllib.loads((ROOT / 'Cargo.toml').read_text(encoding='utf-8'))
    package = cargo['package']
    if package['license'] != 'MIT':
        errors.append('License must remain MIT unless explicitly approved.')
    dependencies = cargo.get('dependencies', {})
    if set(dependencies) != {'windows-sys', 'rustic-calc-app'}:
        errors.append('New runtime dependencies require approval; baseline is only windows-sys.')
    if dependencies.get('windows-sys', {}).get('version') != '0.59':
        errors.append('Review and approve any windows-sys SDK upgrade explicitly.')
    if dependencies.get('rustic-calc-app') != {'path': APP.as_posix(), 'version': package['version']}:
        errors.append('The shared application crate must be local and match the application version.')
    app = tomllib.loads((ROOT / APP / 'Cargo.toml').read_text(encoding='utf-8'))
    if app['package']['version'] != package['version']:
        errors.append('The shared application crate must track the release version.')
    for table in [app, *app.get('target', {}).values()]:
        if any(table.get(key) for key in ('dependencies', 'build-dependencies', 'dev-dependencies')):
            errors.append('The shared application crate must remain dependency-free.')
    for name in EDITION_DEPS:
        check_edition(name, package['version'], errors)
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
    for path in (ROOT / APP / 'src').rglob('*.rs'):
        if re.search(r'windows_sys|crate::platform|std::fs|std::process|std::net', path.read_text(encoding='utf-8')):
            errors.append(f'Shared application must remain pure: {path.relative_to(ROOT)}')
    for name in ['data/msk_templates.tsv', 'data/msk_reference.tsv', 'data/custom_coordinate_reference.tsv',
                 'third_party/geo-ts-msk-LICENSE.txt', 'docs/COORDINATES.md']:
        if not (ROOT / name).is_file():
            errors.append(f'Missing coordinate data/provenance: {name}')
    for path in (ROOT / 'src/views').glob('*.rs'):
        production = path.read_text(encoding='utf-8').split('#[cfg(test)]')[0]
        if re.search(r'\bunsafe\s*(?:\{|fn\b)', production):
            errors.append(f'Unsafe production view code: {path.name}')
        if re.search(r'Lang::(?:Ru|En)\s*=>\s*"', production):
            errors.append(f'Inline localized text belongs in i18n: {path.name}')
    source_dirs = [ROOT / 'src', ROOT / APP / 'src', *(ROOT / 'editions' / name / 'src' for name in EDITION_DEPS)]
    for directory in source_dirs:
        for path in directory.rglob('*.rs'):
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
