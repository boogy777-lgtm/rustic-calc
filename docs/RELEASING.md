# Release procedure

Only maintainers publish releases. Windows x64 is the sole official target.
Do not commit compiled binaries to Git history: attach them to GitHub Releases.

1. Update the root/shared Cargo versions, edition versions, path dependency
   versions, lockfiles and CHANGELOG together. About/PE metadata derive from
   Cargo; the app manifest version/architecture is generated at build time.
2. Run formatting, project checks, Rust tests, isolated CMD tests and screenshots
   for Win32, Slint and GPUI. Record actual results; generated release notes do
   not assert that a test matrix has passed.
3. Build each edition with `scripts/Build-Portable.ps1`, which selects the static
   MSVC CRT for the portable executable. Use the generic x86-64 baseline, never
   `target-cpu=native`.
4. Package and validate all three editions using the locked release procedure
   below. The tag workflow gates the release on all three edition builds/tests
   and uploads the six executables/installers plus the `THIRD-PARTY*` notice
   assets from `dist/editions`.
5. Review the source allowlist, privacy/provenance, hashes, platform support,
   notice bundles, unsigned-artifact notice and installer tasks. Confirm tasks
   remain opt-in and per-user.
6. Create an immutable `v<Cargo version>` tag only after all edition gates pass.
   For 0.5.0 the new tag is `v0.5.0`; do not reuse, overwrite or amend the
   original `v0.4.0` tag/assets. A correction requires a new version/tag.

For local packaging, `-CompilerPath` can select an existing ISCC.exe. Installer
tooling is a build dependency, not part of the standalone EXE. No signing keys
are included. Code signing requires a separately approved secure setup.

## Local three-edition bundle

`scripts/Build-Editions.ps1 -OutputDirectory <new-directory> -CompilerPath <ISCC.exe>`
builds the three Windows x64 frontends separately with locked dependencies.
Each edition must pass its Rust tests and the 66-case CLI screenshot smoke test
before packaging. The script keeps partial evidence in a staging directory
and produces `SHA256SUMS-Editions.txt` for the six executables and three
third-party notice bundles only after all three packages succeed. Packaging
generates `THIRD-PARTY-NOTICES-Win32.txt`,
`THIRD-PARTY-NOTICES-Slint.txt`, and `THIRD-PARTY-NOTICES-GPUI.txt` from
the offline, locked Windows target graphs. `cargo tree --edges normal` is the
feature-resolved target graph; the additional packages in `cargo tree --edges
normal,build` form a conservative build/codegen closure. Dev-only and inactive
optional packages are excluded. Cargo tree entries map to locked metadata by
exact package name, version and repository. Cached legal files are preferred;
version-pinned exceptions in `scripts/License-Notice-Exceptions.psd1` record
the source, attribution rationale and SHA-256 digest. For an upstream release
without a license file, the exception identifies the originally declared
terms and the provenance of any later or redistribution text. A stock license
body does not substitute for publisher attribution.
Missing Cargo license metadata, legal text, provenance, or a matching digest is
a hard packaging failure. Do not bypass it or describe partial results as
complete. Review any author-only attribution and non-Cargo generated/resource
inputs separately before publication. The same edition bundle is installed
under `{app}\licenses\THIRD-PARTY-NOTICES.txt`. The build does not publish,
install, or uninstall any edition.

Win32 retains the legacy `RusticCalc.exe` and `RusticCalc-Setup.exe` filenames,
installation directory and AppId. Slint and GPUI add matching
`RusticCalc-Slint[-Setup].exe` and `RusticCalc-GPUI[-Setup].exe` pairs with
separate AppIds, directories and shortcuts. The CMD task stays Win32-only.
`-Edition` and `-BinaryPath` on `Build-Release.ps1` package one checked release
binary and export its matching `THIRD-PARTY-NOTICES-<Edition>.txt`. Packaging
refuses to replace existing artifacts unless explicitly asked with `-Overwrite`.
The tag workflow, not the local edition script, controls publication.
The manually dispatched CI job accepts `build_editions: true` and runs the
side-by-side installer test on a disposable runner if all three builds pass.
Slint/FemtoVG needs a compatible OpenGL renderer; GPUI needs a compatible
Direct3D 11 device and driver. GPUI screenshots capture the app window only
and fail when the compositor cannot return a usable frame. They never copy
pixels from the surrounding desktop.
Before publishing, review the generated notice bundles against the locked
normal/build dependency graphs, exception provenance and applicable license
terms. The normal graph includes active proc-macro dependencies and is not a
claim that each listed crate is linked into the runtime binary. The build/codegen
closure includes every additional Cargo build-edge package, including some that
may not emit distributed code; it does not enumerate external toolchain/system
SDK components outside Cargo. The bundles preserve package-declared expressions
and discovered legal texts; they are not a legal opinion or proof that upstream
metadata is correct.

## Validation scope

The ordinary CI installs no integration into the runner's real CMD profile;
`Test-CalcCommand.ps1` uses an isolated registry test key. Screenshot tests launch
the GUI with an automatic exit. `Test-Installer.ps1` runs only on a GitHub-hosted
runner or inside Windows Sandbox (pass `-WindowsSandbox` inside the guest).
It keeps CMD integration unchecked and validates install/uninstall and payload
equality. In a guest with a working graphics session, `-VerifyLaunch` also exercises
the installed binaries' CLI screenshot path. Interactive setup and optional tasks
should also be checked in a disposable Windows VM before broader claims.
Never test by overwriting the developer's system calculator.

Before promoting an edition, verify native text entry (including selection,
copy/paste, and IME), Tab/Shift+Tab, Enter/Esc, Ctrl+1…0, Ctrl+U/E, and Ctrl+G
with focus inside a field. Check that drafts survive mode, locale, and theme
changes, and that System theme follows Windows. Use a test account or VM for
clipboard and system changes. Keep software layout checks at
72/96/120/144/168/192/240/288/384 DPI and record physical monitor transitions
separately. A PNG-only or synthetic-input pass does not certify those checks.
`scripts/Test-SlintDpi.ps1 -BinaryPath <Slint.exe>` checks frame geometry for
Standard and WGS84 → MSK using the Slint backend's synthetic scale override.
