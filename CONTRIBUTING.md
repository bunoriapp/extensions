# Contributing to Bunori Extensions

Thank you for your interest in contributing to Bunori Extensions! Contributions of all kinds are welcome, especially new novel sources, bug fixes for existing scrapers, new CMS engine templates, and SDK improvements.

---

## Table of Contents

- [What You Can Contribute](#what-you-can-contribute)
- [Prerequisites & Environment Setup](#prerequisites--environment-setup)
- [Getting Started](#getting-started)
- [Extension Architecture & SDK Overview](#extension-architecture--sdk-overview)
- [Step-by-Step: Adding a New Source](#step-by-step-adding-a-new-source)
  - [1. Scaffold the Source](#1-scaffold-the-source)
  - [2. Configure `manifest.json`](#2-configure-manifestjson)
  - [3. Implement Scraping Logic](#3-implement-scraping-logic)
- [Step-by-Step: Creating a New Template](#step-by-step-creating-a-new-template)
- [Testing Your Extension](#testing-your-extension)
  - [Test Commands & Options](#test-commands--options)
  - [Expected Test Outputs](#expected-test-outputs)
  - [Debugging & Logging](#debugging--logging)
  - [Testing Sites with Anti-Bot / Cloudflare](#testing-sites-with-anti-bot--cloudflare)
- [Packaging & Versioning](#packaging--versioning)
- [Code Guidelines & Best Practices](#code-guidelines--best-practices)
- [Submitting a Pull Request](#submitting-a-pull-request)
- [Commit Message Conventions](#commit-message-conventions)
- [Reporting Issues](#reporting-issues)

---

## What You Can Contribute

- **Add New Novel Sources:** Implement support for popular novel hosting or translation sites.
- **Create Engine Templates:** Build reusable CMS engines (e.g., Madara, NovelFire, ReadNovelFull, WpManga).
- **Fix Broken Sources:** Update outdated CSS selectors, API endpoints, or chapter pagination logic.
- **Improve the SDK:** Enhance HTML parsing helpers, error reporting, or FFI communication.
- **Improve Documentation & Tooling:** Improve developer CLI scripts, test harnesses, and guides.

---

## Prerequisites & Environment Setup

### 1. Rust Toolchain (1.75+)
Extensions are compiled to WebAssembly using Rust. Make sure you have Rust installed along with the `wasm32-unknown-unknown` target:

```bash
# Add WebAssembly compilation target
rustup target add wasm32-unknown-unknown
```

### 2. Python Environment (3.10+)
The testing and packaging tools use Python. Set up a virtual environment and install the required dependencies:

```bash
# Create and activate virtual environment
python3 -m venv .venv
source .venv/bin/activate

# Install test runner dependencies (wasmtime and curl-cffi)
pip install wasmtime curl_cffi
```

---

## Getting Started

### 1. Fork and Clone
```bash
git clone https://github.com/<your_username>/extensions.git
cd extensions
```

### 2. Create a Topic Branch
Use a descriptive branch name:
```bash
git checkout -b feat/add-lunarletters-source
# or
git checkout -b fix/novelfire-chapter-parsing
```

### 3. Verify Your Environment
Run a workspace check to verify that all crates compile:
```bash
cargo check --workspace
```

---

## Extension Architecture & SDK Overview

Every Bunori extension implements the `Source` trait defined in `bunori-sdk`:

```rust
use bunori_sdk::*;

pub trait Source {
    fn metadata(&self) -> SourceMetadata;
    fn search(&self, query: &str, page: i32) -> Result<Vec<SearchResultDto>, String>;
    fn get_novel_details(&self, novel_url: &str) -> Result<NovelDto, String>;
    fn get_chapter_content(&self, chapter_url: &str) -> Result<Option<String>, String>;
    fn get_listings(&self) -> Vec<ListingDto>;
    fn get_listing_novels(&self, listing_id: &str, page: i32) -> Result<Vec<SearchResultDto>, String>;
}
```

### Key Models & DTOs

| DTO | Description | Key Fields |
| :--- | :--- | :--- |
| `SourceMetadata` | Extension information | `id`, `name`, `version`, `baseUrl`, `lang`, `iconUrl` |
| `SearchResultDto` | Search or listing result item | `url`, `title`, `cover_url`, `author` |
| `NovelDto` | Full novel details | `url`, `title`, `author`, `cover_url`, `description`, `status`, `genres`, `chapters` |
| `ChapterDto` | Individual chapter entry | `url`, `title`, `index`, `release_date`, `scanlation` |
| `ListingDto` | Browse filter definition | `id` (e.g. `"popular"`, `"latest"`), `name` |

### Built-in Host Functions
Extensions do not perform raw socket networking. Instead, `bunori-sdk` provides safe host functions:
- `host::get(url, headers)`: Performs an HTTP GET request through the host client.
- `host::post(url, body, headers)`: Performs an HTTP POST request.
- `host::document(url, headers)`: Convenience helper that fetches a URL and parses it into a `scraper::Html` document in one step.
- `host::time_ms()`: Returns current Unix timestamp in milliseconds.
- `log_info!`, `log_debug!`, `log_warn!`, `log_error!`: Logs output to the host console.

---

## Step-by-Step: Adding a New Source

### 1. Scaffold the Source

Use `tools/source.py` to create the crate structure under `sources/<source_id>/`.

#### Option A: Using an Existing Template Engine (Recommended if site uses standard CMS)
If the site runs on a known CMS like **Madara**, **NovelFire**, or **ReadNovelFull**:

```bash
python tools/source.py --name "WuxiaWorld Site" --url "https://wuxiaworld.site" --template madara
```

This creates:
- `sources/wuxiaworldsite/manifest.json`
- `sources/wuxiaworldsite/Cargo.toml` (with dependency on `madara-template`)
- `sources/wuxiaworldsite/src/lib.rs` (pre-wired to delegate to `MadaraEngine`)

#### Option B: Creating a Standalone Custom Source
If the site has a custom design or proprietary API:

```bash
python tools/source.py --name "Novel Hi" --url "https://novelhi.com"
```

---

### 2. Configure `manifest.json`

Check `sources/<source_id>/manifest.json` and ensure all fields are accurate:

```json
{
  "id": "wuxiaworldsite",
  "name": "WuxiaWorld.Site",
  "version": "0.0",
  "apiVersion": 1,
  "lang": "en",
  "baseUrl": "https://wuxiaworld.site",
  "authors": ["zenit"],
  "isDeprecated": false,
  "iconUrl": "https://wuxiaworld.site/favicon.ico",
  "webviewNeeded": false,
  "runnerConcurrency": 3,
  "runnerCooldown": 1000,
  "maxAttempts": 3
}
```

- **`id`**: Unique lowercase identifier with underscores only (must match directory name).
- **`name`**: Human-readable name of the novel source.
- **`version`**: Extension version (`0.0`). Always increment when publishing changes.
- **`authors`**: Array of maintainer handles / names credited for developing the extension (e.g. `["zenit", "contributor2"]`).
- **`isDeprecated`**: Boolean flag indicating if the source is discontinued / shut down (default: `false`).
- **`deprecationReason`** *(optional)*: Explanation when `isDeprecated: true` (e.g. `"Website permanently closed"`).
- **`suggestedAlternative`** *(optional)*: ID of a replacement extension (e.g. `"novelbins"`).
- **`lang`**: ISO 639-1 language code (e.g. `"en"`, `"es"`, `"ar"`, `"zh"`, `"tr"`).
- **`baseUrl`**: Canonical root URL without trailing slash.
- **`webviewNeeded`**: Set to `true` if Cloudflare Turnstile or JS evaluation is strictly required.
- **`runnerConcurrency`**: Recommended concurrent requests (default: `3`).
- **`runnerCooldown`**: Delay in milliseconds between requests (default: `1000`).
- **`maxAttempts`**: Maximum retry attempts on network error (default: `3`).

---

### 3. Implement Scraping Logic

Edit `sources/<source_id>/src/lib.rs` to implement or customize the required methods.

#### Implementing Search:
```rust
fn search(&self, query: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {
    let meta = self.metadata();
    let formatted_query = query.replace(' ', "+");
    let url = format!("{}/search?keyword={}&page={}", meta.base_url, formatted_query, page);
    let doc = host::document(&url, None)?;

    let item_sel = Selector::parse(".novel-item").map_err(|e| e.to_string())?;
    let title_sel = Selector::parse("h3.title a").map_err(|e| e.to_string())?;
    let cover_sel = Selector::parse(".cover img").map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for el in doc.select(&item_sel) {
        let Some(link) = el.select(&title_sel).next() else { continue; };
        let title = link.text().collect::<Vec<_>>().join("").trim().to_string();
        let Some(href) = link.value().attr("href") else { continue; };

        let novel_url = if href.starts_with("http") { href.to_string() } else { format!("{}{}", meta.base_url, href) };
        let cover_url = el.select(&cover_sel).next()
            .and_then(|img| img.value().attr("src").or_else(|| img.value().attr("data-src")))
            .map(|s| if s.starts_with("http") { s.to_string() } else { format!("{}{}", meta.base_url, s) });

        if !title.is_empty() && !novel_url.is_empty() {
            results.push(SearchResultDto {
                url: novel_url,
                title,
                cover_url,
                author: None,
            });
        }
    }

    Ok(results)
}
```

#### Implementing Novel Details & Chapters:
- **Title & Metadata:** Extract title, description, cover image URL, author, status, and genres.
- **Chapter Ordering:** Chapters must be indexed starting from `1` (earliest chapter) to `N` (latest chapter). If the site displays the newest chapter first, make sure to reverse the vector.
- **Relative URLs:** Always convert relative URLs to absolute URLs using `meta.base_url`.

#### Implementing Chapter Content:
- Select the reading text container (e.g. `#chapter-content`, `.reading-content`).
- **Clean Unwanted Elements:** Strip ads, scripts, `<style>`, `<iframe>`, and tracking code.

```rust
fn get_chapter_content(&self, chapter_url: &str) -> Result<Option<String>, String> {
    let doc = host::document(chapter_url, None)?;
    let sel = Selector::parse("#chapter-content").map_err(|e| e.to_string())?;
    
    let Some(el) = doc.select(&sel).next() else {
        return Ok(None);
    };

    let mut content = el.inner_html();
    for tag in &["script", "style", "ins", "iframe", "button"] {
        let open = format!("<{}", tag);
        let close = format!("</{}>", tag);
        while let Some(s) = content.find(&open) {
            if let Some(e) = content[s..].find(&close) {
                content.replace_range(s..s + e + close.len(), "");
            } else {
                break;
            }
        }
    }

    Ok(Some(content.trim().to_string()))
}
```

---

## Step-by-Step: Creating a New Template

If multiple novel websites share the same CMS or HTML layout, create an engine template under `templates/<template_name>/`:

```bash
python tools/template.py --name "MyCMS" --struct-name "MyCmsEngine"
```

1. Define configurable selectors and paths on the engine struct in `templates/mycms/src/lib.rs`.
2. Provide default values in `MyCmsEngine::new(base_url)`.
3. Implement `search`, `get_novel_details`, `get_chapter_content`, and `get_listing_novels` on the engine struct.
4. Source extensions can then reuse the engine by instantiating it and overriding only site-specific selectors or flags.

---

## Testing Your Extension

We provide a testing harness (`tools/test.py`) that loads your compiled WASM module into a standalone `wasmtime` runner and simulates the host environment.

### 1. Compile to WebAssembly
Before running tests, compile your source crate:

```bash
cargo build -p <extension_id> --target wasm32-unknown-unknown --release
```

### 2. Run the Test Harness

```bash
# 1. Verify Metadata
python tools/test.py <extension_id> --metadata

# 2. Test Search
python tools/test.py <extension_id> --search "sword" --page 1

# 3. Test Novel Details and Chapter List
python tools/test.py <extension_id> --details "<novel_url>"

# 4. Test Chapter Text Content
python tools/test.py <extension_id> --chapter "<chapter_url>"

# 5. Test Supported Browse Listings
python tools/test.py <extension_id> --listings
python tools/test.py <extension_id> --listing-novels "<listing_id>" --page 1
```

---

### Expected Test Outputs

#### Example: Metadata Test (`--metadata`)
```json
--- [Metadata] ---
{
  "id": "wuxiaworldsite",
  "name": "WuxiaWorld.Site",
  "version": "0.0",
  "apiVersion": 1,
  "lang": "en",
  "baseUrl": "https://wuxiaworld.site",
  "iconUrl": "https://wuxiaworld.site/favicon.ico",
  "webviewNeeded": false,
  "runnerConcurrency": 3,
  "runnerCooldown": 1000,
  "maxAttempts": 3
}
```

#### Example: Novel Details Test (`--details`)
```
--- [Novel Details: 'https://wuxiaworld.site/novel/martial-peak/'] ---
Title: Martial Peak
Author: Momo
Genres: Action, Adventure, Cultivation, Fantasy, Martial Arts
Total Chapters: 6009
  First: Chapter 1 -> https://wuxiaworld.site/novel/martial-peak/chapter-1/
  Last:  Chapter 6009 -> https://wuxiaworld.site/novel/martial-peak/chapter-6009/
```

#### Example: Chapter Content Test (`--chapter`)
```
--- [Chapter Content: 'https://wuxiaworld.site/novel/martial-peak/chapter-1/'] ---
Length: 8420 characters
Preview (first 400 chars):
<p>Yang Kai swept the courtyard with a broom...</p>
```

---

### Debugging & Logging

You can output debug messages from your Rust code during test runs using the logging macros:

```rust
log_info!("Fetching novel details for {}", novel_url);
log_debug!("Found {} chapters in DOM", chapters.len());
```

When you run `python tools/test.py`, log messages will appear in real time:
```
 [INFO] Fetching novel details for https://wuxiaworld.site/novel/martial-peak/
 [DEBUG] Found 6009 chapters in DOM
```

---

### Testing Sites with Anti-Bot / Cloudflare

If a site requires Cloudflare clearance or authentication cookies during local testing, pass them using the `--cookie` and `--user-agent` flags:

```bash
python tools/test.py <extension_id> \
  --search "sword" \
  --cookie "cf_clearance=abc123...; session=xyz;" \
  --user-agent "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36..."
```

---

## Packaging & Versioning

Extensions are distributed as `.bext` archives containing `manifest.json`, `source.wasm`, `CHANGELOG.md`, and pre-compiled AOT artifacts.

### Versioning Rules
Whenever you update an extension, increment the `"version"` field in `manifest.json`:
- **Bug fix (e.g. selector fix):** Bump patch version (`0.0` -> `0.1`).
- **Feature update (e.g. added listings/filters):** Bump minor version (`0.0` -> `1.0`).

### Maintaining `CHANGELOG.md`
Every extension maintains its own `sources/<extension_id>/CHANGELOG.md` following [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format.

Whenever you bump the version in `manifest.json`, you **must** add a new entry to `sources/<extension_id>/CHANGELOG.md`:

```markdown
# Changelog - Royal Road

## [0.2] - 2026-09-21
- Added search genre filtering support.
- Fixed chapter text unescape formatting.

## [0.1] - 2026-09-15
- Initial release.
```

The packager extracts the latest release notes from `CHANGELOG.md` and displays them in the Bunori App update prompt.

---

## Extension Deprecation & Discontinuation Policy

If a novel website permanently shuts down, rebrands, or becomes unusable due to intractable anti-bot protection:

1. **Do not delete the source directory immediately** (doing so would orphan existing user installs).
2. Set `"isDeprecated": true` in `sources/<extension_id>/manifest.json`.
3. Add a clear `"deprecationReason"` explaining the status.
4. If a successor or alternate source exists, provide `"suggestedAlternative": "<replacement_id>"`.
5. Bump the patch version in `manifest.json` and document the deprecation in `CHANGELOG.md`.

Example deprecated `manifest.json`:
```json
{
  "id": "novelfire",
  "name": "Novel Fire",
  "version": "0.4",
  "apiVersion": 1,
  "lang": "en",
  "baseUrl": "https://novelfire.net",
  "authors": ["zenit"],
  "isDeprecated": true,
  "deprecationReason": "Domain closed by owner. Replaced by Novel Bins.",
  "suggestedAlternative": "novelbins"
}
```

---

## Authors & Attribution

We believe in giving full credit to contributors.
- Add your GitHub handle or name to the `"authors"` list in `manifest.json`.
- If multiple developers contribute to an extension, append additional handles to the list:
  ```json
  "authors": ["primary_dev", "contributor_name"]
  ```

---

### Packaging Locally
```bash
# Package a single extension
python tools/package.py --single <extension_id> --no-aot

# Package all extensions
python tools/package.py --compile-all
```

---

## Code Guidelines & Best Practices

1. **Avoid `unwrap()` or `expect()` in Scraping Logic:**
   HTML structure changes frequently. Use `?`, `if let`, or default fallbacks instead of crashing the WASM runtime.
2. **Clean Chapter Content:**
   Ensure chapter text does not contain injected advertisement blocks, hidden spam keywords, or unclosed script tags.
3. **Keep Dependencies Minimal:**
   Do not add heavy external crates to extension `Cargo.toml`. Rely on `bunori-sdk`, `scraper`, and `serde_json`.
4. **Isolate Site Logic:**
   Do not modify shared SDK or template behavior to fix a single non-standard website; customize the source implementation instead.

---

## Submitting a Pull Request

Before opening a pull request:
1. Ensure your extension passes `cargo check --workspace`.
2. Test search, novel details, chapter extraction, and listings using `python tools/test.py`.
3. Verify that `manifest.json` version has been bumped if modifying an existing source.
4. Add corresponding release notes in `sources/<extension_id>/CHANGELOG.md`.
5. Include your handle in `"authors"` in `manifest.json` for credit.
6. Remove temporary debugging logs and test scratch files.

---

## Commit Message Conventions

Use concise, conventional commit prefixes:

- `feat: add wuxiaworldsite source`
- `fix: update novelfire chapter ajax selector`
- `refactor: extract common madara pagination logic`
- `docs: update contributing guide with test examples`

---

## Reporting Issues

- If a novel source stops working due to website layout changes, please [open an issue](https://github.com/bunoriapp/extensions/issues) with:
  - **Source Name**
  - **Novel URL**
  - **Chapter URL** (if applicable)
  - **Error output from `python tools/test.py`**
- For general discussions and community help, join our **[Discord Server](https://discord.gg/A6cY7pN6Y)**.
