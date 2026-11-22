# Provenance and third-party notices

Rustic Calc is an independent Rust implementation of calculator behavior.
Compatibility research informed some arithmetic/UI decisions; research captures,
third-party executables, MUI/resource dumps and decompiled listings are not part
of this repository or its releases. Do not add them to a PR.

Mathematical formulas, unit factors and standard E24 values are encoded as
self-contained numerical data. No external resource bundle is loaded at runtime.
The project is not affiliated with or endorsed by Microsoft.

The application's original source and included project artwork are distributed under MIT.
The Win32 edition uses `windows-sys`, provided by the windows-rs project under
MIT OR Apache-2.0; its transitive Windows target crates retain their upstream
licenses. The shared application crate is first-party and dependency-free.
Installer tooling retains its own upstream license and is not a runtime
dependency of the portable calculator.

The separately built Slint edition uses Slint 1.18.1 under the
[Slint Royalty-free Desktop, Mobile, and Web Applications License 2.0](https://github.com/slint-ui/slint/blob/v1.18.1/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md).
Its desktop distribution requires the Slint attribution described in that license.
The GPUI edition uses [Zed's GPUI 0.2.2](https://crates.io/crates/gpui/0.2.2),
copyright 2022–2025 Zed Industries, Inc., licensed under Apache-2.0. Each
Windows edition's release packaging generates a `THIRD-PARTY-NOTICES-<Edition>.txt`
bundle from its offline, locked Windows target graph. `cargo tree --edges normal`
defines the active target dependency graph; the additional packages from
`cargo tree --edges normal,build` are kept as a separate, conservative
build/codegen closure. Dev dependencies and inactive optional packages are
excluded. Cargo tree entries are mapped to locked package metadata by exact
name, version and repository.

The generator includes legal files from cached crate archives. Where an exact
crate archive omits them, `scripts/License-Notice-Exceptions.psd1` pins the
upstream commit and verifies the vendored text's SHA-256. It never substitutes
a generic license body for missing publisher terms or copyright attribution.
Cargo `authors` metadata is reported as authors, never rewritten as a copyright
notice. Generation fails if license metadata, legal text, exception provenance
or a digest is missing. Bundles also contain this notice, the project license
and the MSK catalogue license. They are not a legal determination or release
sign-off: review upstream attribution and any non-Cargo generated/resource
inputs before publication.

## MSK community definitions (Apache-2.0)

`data/msk_templates.tsv` is adapted from `index.ts` in
[geo-ts/msk](https://github.com/geo-ts/msk), revision
`946049d80f0f4b4956a53383961c8c6f4431169f` (2024-07-27).
The definitions retain their upstream Apache License 2.0, reproduced in
`third_party/geo-ts-msk-LICENSE.txt` and embedded in the standalone executable's
coordinate-mode help (Ctrl+G, F2). Upstream contains no separate NOTICE file or
specific copyright attribution beyond its license; no endorsement is implied.

Changes are declared in the TSV header: serialization to TSV, stable regional
keys, whitespace normalization and grouping Krasnoyarsk city/local systems under
region 24. Numeric parameters are unchanged. This is a community catalogue, not
an official registry or a claim of geodetic accuracy. The application's MIT
license does not relicense the upstream catalogue.

`data/msk_reference.tsv` and `data/custom_coordinate_reference.tsv` contain generated numeric test results from proj4.js
2.12.1, not the library's source. The library is used only by an optional developer
fixture generator; it is not bundled or loaded by the application. See
`docs/COORDINATES.md` for reproducible commands and sources.
