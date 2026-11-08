# Release procedure

Only maintainers publish releases. Windows x64 is the sole official target.
Do not commit compiled binaries to Git history: attach them to GitHub Releases.

1. Update Cargo version, Cargo.lock and CHANGELOG. About/PE metadata derive from
   Cargo; the app manifest version/architecture is generated at build time.
2. Run formatting, project checks, Rust tests, isolated CMD tests and screenshots.
3. Build release with the generic x86-64 baseline. Never use target-cpu=native.
4. Install Inno Setup 6 on the build machine and run `scripts/Build-Release.ps1`.
   The portable binary and installer must contain the same verified EXE.
5. Review the source allowlist, privacy/provenance, hashes, platform support and
   unsigned-artifact notice. Confirm installer tasks are opt-in and per-user.
6. After approval push an immutable `v<Cargo version>` tag. The release workflow
   checks the version and publishes `RusticCalc.exe`, `RusticCalc-Setup.exe` and
   `SHA256SUMS.txt`. Do not overwrite existing version tags/assets silently.

For local packaging, `-CompilerPath` can select an existing ISCC.exe. Installer
tooling is a build dependency, not part of the standalone EXE. No signing keys
are included. Code signing requires a separately approved secure setup.

## Validation scope

The ordinary CI installs no integration into the runner's real CMD profile;
`Test-CalcCommand.ps1` uses an isolated registry test key. Screenshot tests launch
the GUI with an automatic exit. `Test-Installer.ps1` runs only on a disposable
GitHub Actions runner, with CMD integration explicitly unchecked, and validates
install/uninstall plus payload equality. Interactive setup and optional tasks
should also be checked in a disposable Windows VM before broader claims.
Never test by overwriting the developer's system calculator.
