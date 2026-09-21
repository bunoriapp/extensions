<div align="center">

# Bunori Extensions

### Official WebAssembly (WASM) Extension Repository for Bunori

<br />

[![License](https://img.shields.io/github/license/bunoriapp/extensions?style=for-the-badge&labelColor=0d1117)](https://github.com/bunoriapp/extensions/blob/main/LICENSE)
[![Discord](https://img.shields.io/badge/Discord-%235865F2.svg?style=for-the-badge&logo=discord&logoColor=white&labelColor=0d1117)](https://discord.gg/A6cY7pN6Y)

<br />

</div>

---

## Usage Instruction

To browse and install extensions in the Bunori app, add the **Repository URL** to your application settings:

```
https://bunoriapp.github.io/extensions/index.min.json
```

---

## Overview

**Bunori Extensions** is the repository containing source crawlers for the **[Bunori](https://github.com/bunoriapp/bunori)** ecosystem.

Unlike traditional novel reader crawlers written in JavaScript or platform-specific JVM bytecode, Bunori extensions are compiled to **WebAssembly (WASM)** and packaged into lightweight `.bext` archives. Providing:
- **Cross-platform support** across Android, Desktop, and CLI runners.
- **Fast, memory-safe execution** in isolated WASM runtimes.
- **Host-managed networking** with built-in Network bypass, HTTP connection pooling, and cookie preservation.

---

## Architecture & Bext Structure

Each extension is an independent Rust crate compiled to `wasm32-unknown-unknown`. Network I/O and logging are assigned to the host application through a Foreign Function Interface (FFI).

```
┌────────────────────────────────────────────────────────┐
│                      Bunori Host                       │
│             (Android App / Desktop Client)             │
│                                                        │
│   ┌─────────────────────┐    ┌─────────────────────┐   │
│   │     WASM Engine     │    │     HttpClient      │   │
│   │                     │                          │   │
│   └──────────▲──────────┘    └──────────▲──────────┘   │
└──────────────┼──────────────────────────┼──────────────┘
               │      host_http() FFI     │
┌──────────────▼──────────────────────────▼──────────────┐
│                    Extension (.bext)                   │
│   ├── manifest.json                                    │
│   ├── source.wasm                                      │
│   └── artifacts/                                       │
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

#### Web Assembly
- Compiled from memory-safe Rust to sandboxed `.wasm` bytecode.
- Near-native execution performance with optional AOT compilation.

</td>
    <td width="50%" valign="top">

#### Reusable Templates
- Shared template engines.
- Scaffold a fully functional novel source with only a few lines of configuration.

</td>
  </tr>
  <tr>
    <td width="50%" valign="top">

#### Sandboxing
- Extensions never handle raw TLS or sockets directly.
- The host transparently manages User-Agents, cookies, and any other challenges.

</td>
    <td width="50%" valign="top">

#### Built-in Test Suite
- Comprehensive Python test harness (`tools/test.py`) with native Wasmtime integration.
- Test metadata, live searches, novel details, and chapter text extraction in seconds.

</td>
  </tr>
  <tr>
    <td width="50%" valign="top">

#### Automated deployment pipeline
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

<div align="center">

<br/>

**Made with ❤️ for the [Bunori](https://github.com/bunoriapp/bunori) Community**

</div>
