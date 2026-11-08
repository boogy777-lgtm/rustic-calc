# Provenance and third-party notices

Rustic Calc is an independent Rust implementation of calculator behavior.
Compatibility research informed some arithmetic/UI decisions; research captures,
third-party executables, MUI/resource dumps and decompiled listings are not part
of this repository or its releases. Do not add them to a PR.

Mathematical formulas, unit factors and standard E24 values are encoded as
self-contained numerical data. No external resource bundle is loaded at runtime.
The project is not affiliated with or endorsed by Microsoft.

The application's source and included project artwork are distributed under MIT.
The only direct Rust dependency, `windows-sys`, is provided by the windows-rs
project under MIT OR Apache-2.0; its transitive Windows target crates retain
their upstream licenses. Installer tooling retains its own upstream license and
is not a runtime dependency of the portable calculator.
