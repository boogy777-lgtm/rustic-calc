# Security

Report vulnerabilities privately through GitHub's repository security advisory
facility when enabled. Do not post secrets or working exploit payloads in a
public issue. For ordinary bugs use the issue tracker.

Only the latest published Windows x64 release is maintained. Linux/macOS builds
and native x86/ARM64 artifacts are not currently released or supported.

The app has no telemetry, network updater or downloaded runtime resources.
Official release artifacts are currently unsigned; verify their GitHub origin
and SHA-256. Unsigned does not mean SmartScreen/reputation warnings can be ignored.

Optional CMD integration writes only a reversible current-user AutoRun hook and
user-local files. It must never replace System32 files or change HKLM/persistent
PATH. Registry changes are opt-in; tests must use isolated keys.
