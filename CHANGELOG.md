# Changelog - Bunori Extensions Repository

All notable changes to the Bunori extensions ecosystem, SDK, and tooling will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- Extension deprecation lifecycle support (`isDeprecated`, `deprecationReason`, `suggestedAlternative`) in SDK models, manifest schema, and packager.
- Extension author and maintainer attribution support (`authors` array in `manifest.json` and `repo/index.json`).
- Per-extension `CHANGELOG.md` support with automatic latest release notes extraction into `repo/index.json` and `.bext` packages.
- Interactive and CLI author options in `tools/source.py` scaffolding tool.
- Comprehensive deprecation policy, author crediting, and changelog guidelines in `CONTRIBUTING.md`.

## [1.0.0] - 2026-09-21
- Initial release of the Bunori WebAssembly Extension SDK and packaging pipeline.
- 13 initial novel source extensions and 3 CMS engine templates (Madara, NovelFire, ReadNovelFull).
