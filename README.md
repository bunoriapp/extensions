<div align="center">

# Bunori Extensions

### Official WebAssembly (WASM) Extension Repository for Bunori

<br />

[![License](https://img.shields.io/github/license/bunoriapp/extensions?style=for-the-badge&labelColor=0d1117)](https://github.com/bunoriapp/extensions/blob/main/LICENSE)
[![Discord](https://img.shields.io/badge/Discord-%235865F2.svg?style=for-the-badge&logo=discord&logoColor=white&labelColor=0d1117)](https://discord.gg/A6cY7pN6Y)

<br />

</div>

---

## Overview

**Bunori Extensions** is the official repository containing source crawlers for the **[Bunori](https://github.com/bunoriapp/bunori)** light novel reader and crawler ecosystem.

Unlike traditional novel reader crawlers written in JavaScript or platform-specific JVM bytecode, Bunori extensions are compiled to **WebAssembly (WASM)** and packaged into lightweight `.bext` archives. This provides:
- **True cross-platform support** across Android, Desktop, and CLI runners.
- **Fast, memory-safe execution** in isolated WASM runtimes.
- **Host-managed networking** with built-in Cloudflare bypass, HTTP connection pooling, and cookie preservation.

---

## Architecture

Each extension is an independent Rust crate compiled to `wasm32-unknown-unknown`. Network I/O and logging are delegated to the host application through a standardized Foreign Function Interface (FFI).

```
┌────────────────────────────────────────────────────────┐
│                      Bunori Host                       │
│             (Android App / Desktop Client)             │
│                                                        │
│   ┌─────────────────────┐    ┌─────────────────────┐   │
│   │     WASM Engine     │    │     HttpClient      │   │
│   │ (Wasmtime / WAMR)   │    │  (Cloudflare + TLS) │   │
│   └──────────▲──────────┘    └──────────▲──────────┘   │
└──────────────┼──────────────────────────┼──────────────┘
               │      host_http() FFI     │
┌──────────────▼──────────────────────────▼──────────────┐
│                    Extension (.bext)                   │
│   ├── manifest.json       (Metadata & capabilities)    │
│   ├── source.wasm         (WASM bytecode)              │
│   └── artifacts/          (Optional AOT pre-compiled)  │
│       ├── arm64-v8a/extension.aot                      │
│       └── x86_64/extension.aot                         │
└────────────────────────────────────────────────────────┘
```

---

<div align="center">

# Features

</div>

<table>
  <tr>
    <td width="50%" valign="top">

#### WebAssembly Powered
- Compiled from memory-safe Rust to sandboxed `.wasm` bytecode.
- Near-native execution performance with optional AOT compilation.

</td>
    <td width="50%" valign="top">

#### Reusable Templates
- Shared CMS template engines (**Madara**, **NovelFire**, **ReadNovelFull**).
- Scaffold a fully functional novel source with only a few lines of configuration.

</td>
  </tr>
  <tr>
    <td width="50%" valign="top">

#### Host Network Sandboxing
- Extensions never handle raw TLS or sockets directly.
- The host transparently manages User-Agents, cookies, and Cloudflare challenges.

</td>
    <td width="50%" valign="top">

#### Built-in Test Suite
- Comprehensive Python test harness (`tools/test.py`) with native Wasmtime integration.
- Test metadata, live searches, novel details, and chapter text extraction in seconds.

</td>
  </tr>
  <tr>
    <td width="50%" valign="top">

#### Automated CI/CD
- Automated GitHub Actions pipeline builds `.bext` packages and publishes `index.json`.
- Automatic version bumping and package distribution via GitHub Pages.

</td>
    <td width="50%" valign="top">

#### Developer Tooling
- Dedicated CLI scaffolding tools for new sources and engine templates.
- Clear error reporting and type-safe SDK interfaces.

</td>
  </tr>
</table>

---

## Repository Layout

```
BunoriExtensions/
├── sdk/                     # Bunori Rust SDK (bunori-sdk)
│   ├── src/
│   │   ├── abi.rs           # FFI macros & memory management
│   │   ├── host.rs          # Host HTTP, logging & document helpers
│   │   ├── models.rs        # Data Transfer Objects (NovelDto, ChapterDto, etc.)
│   │   └── lib.rs
├── templates/               # Reusable engine templates
│   ├── madara/              # WordPress Madara theme engine
│   ├── novelfire/           # NovelFire CMS engine
│   └── readnovelfull/       # ReadNovelFull CMS engine
├── sources/                 # Individual novel source extensions
│   ├── asianovel/
│   ├── lunarletters/
│   ├── novelbins/
│   ├── novelfire/
│   ├── novelfull/
│   ├── readnovelfull/
│   ├── wuxiaworldsite/
│   └── ...
├── tools/                   # Developer CLI tools
│   ├── source.py        # Scaffold new source extensions
│   ├── template.py      # Scaffold new template engines
│   ├── test.py              # Local WASM execution & test harness
│   └── package.py           # WASM & AOT packaging script
├── repo/                    # Built .bext packages and repository index
├── Cargo.toml               # Workspace manifest
├── CONTRIBUTING.md          # Step-by-step contribution guide
└── README.md
```

---

## 🚀 Quick Start

### Prerequisites
1. **Rust** (1.75+) with WebAssembly compilation target:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```
2. **Python** (3.10+) with test dependencies:
   ```bash
   pip install wasmtime curl_cffi
   ```

---

### Creating a New Source

Use `tools/new_source.py` to create a new source extension.

- **Using an existing template engine (e.g. Madara):**
  ```bash
  python tools/source.py --name "WuxiaWorld Site" --url "https://wuxiaworld.site" --template madara
  ```
- **Creating a standalone source:**
  ```bash
  python tools/source.py --name "Novel Hi" --url "https://novelhi.com"
  ```

---

### Testing Your Extension

1. **Build the WASM binary:**
   ```bash
   cargo build -p wuxiaworldsite --target wasm32-unknown-unknown --release
   ```

2. **Run tests via the test harness:**
   ```bash
   # Test metadata
   python tools/test.py wuxiaworldsite --metadata

   # Test search
   python tools/test.py wuxiaworldsite --search "cultivation"

   # Test novel details & chapter list
   python tools/test.py wuxiaworldsite --details "https://wuxiaworld.site/novel/martial-peak/"

   # Test chapter text content
   python tools/test.py wuxiaworldsite --chapter "https://wuxiaworld.site/novel/martial-peak/chapter-1/"

   # Test supported listings
   python tools/test.py wuxiaworldsite --listings
   ```

---

### Packaging Extensions

Package your extension into a `.bext` archive and update the local repository index:

```bash
# Package a single extension
python tools/package.py --single wuxiaworldsite --no-aot

# Package all extensions for release
python tools/package.py --compile-all
```

The output `.bext` archives and `index.json` catalog will be generated in `repo/`.

---

## Using in Bunori

To browse and install extensions in the Bunori app, add the repository URL to your application settings:

```
https://bunoriapp.github.io/extensions/index.min.json
```

---

## Contributing

We welcome contributions of all kinds, including new source crawlers, engine templates, bug fixes, and documentation improvements.

Please read our **[Contributing Guide](CONTRIBUTING.md)** for detailed step-by-step instructions on implementing sources, extracting CSS selectors, and testing.

---

<div align="center">

<br/>

**Made with ❤️ for the [Bunori](https://github.com/bunoriapp/bunori) Community**

</div>
