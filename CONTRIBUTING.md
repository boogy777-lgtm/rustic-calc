# Contributing

Bug fixes, documentation and genuine backend/architecture ports are welcome.
For larger modules open an issue before implementation; follow
[AGENTS.md](AGENTS.md) and [the module SDK](docs/MODULE_SDK.md).

1. Create a focused branch and describe the behavior being changed.
2. Add regression coverage; run the commands in README and relevant script tests.
3. Include before/after screenshots for UI changes, listing language/theme/DPI.
4. State platform/toolchain, physical vs synthetic tests and remaining limitations.
5. Submit a PR. Do not commit target/dist, investigation files, private paths or
   credentials. Release binaries are GitHub Release assets, not Git source blobs.

By contributing, you agree your original contribution is provided under the
project's MIT license. Disclose third-party provenance/licenses; do not submit
proprietary resource dumps or code. Maintainers review dependency, OS backend,
registry/installer and public-contract changes explicitly.
