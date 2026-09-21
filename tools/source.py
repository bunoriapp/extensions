"""
Scaffolding CLI for creating a new Bunori extension source.
Generates manifest.json, Cargo.toml, and src/lib.rs.

Usage:
  python tools/source.py
  python tools/source.py -n "WuxiaWorld Site" -u "https://wuxiaworld.site" --template madara
  python tools/source.py --name "Novel Hi" --url "https://novelhi.com" --lang en --id novelhi
"""

import argparse
import datetime
import json
import re
import subprocess
import sys
from pathlib import Path


def get_default_author() -> str:
    try:
        res = subprocess.run(["git", "config", "user.name"], capture_output=True, text=True, check=False)
        return res.stdout.strip()
    except Exception:  # noqa: BLE001
        return ""


def sanitize_id(name: str) -> str:
    """Derive a valid Cargo crate / extension ID from the name."""
    s = name.lower()
    s = re.sub(r"[^a-z0-9_]+", "", s)
    return s


def to_pascal_case(name: str) -> str:
    """Convert a name to a valid Rust PascalCase struct name."""
    words = re.findall(r"[a-zA-Z0-9]+", name)
    pascal = "".join(w.capitalize() for w in words)
    if not pascal or not pascal[0].isalpha():
        pascal = "My" + pascal
    return pascal


def create_source(
    name: str,
    base_url: str,
    ext_id: str | None = None,
    lang: str = "en",
    authors: list[str] | str | None = None,
    template: str | None = None,
):
    project_root = Path(__file__).resolve().parent.parent
    sources_dir = project_root / "sources"

    if not ext_id:
        ext_id = sanitize_id(name)

    if not re.match(r"^[a-z0-9_]+$", ext_id):
        raise ValueError(
            f"Invalid extension ID '{ext_id}'. Must contain only lowercase letters, digits, and underscores."
        )

    target_dir = sources_dir / ext_id
    if target_dir.exists():
        print(f"Error: Directory already exists at {target_dir}", file=sys.stderr)
        sys.exit(1)

    # Normalize base_url (remove trailing slash)
    base_url = base_url.strip().rstrip("/")
    if not base_url.startswith("http://") and not base_url.startswith("https://"):
        base_url = "https://" + base_url

    # Normalize authors list
    if isinstance(authors, str):
        author_list = [a.strip() for a in authors.split(",") if a.strip()]
    elif isinstance(authors, list):
        author_list = [str(a).strip() for a in authors if str(a).strip()]
    else:
        author_list = []

    pascal_name = to_pascal_case(name)

    # 1. Create directories
    src_dir = target_dir / "src"
    src_dir.mkdir(parents=True, exist_ok=True)

    # 2. Generate manifest.json
    manifest = {
        "id": ext_id,
        "name": name,
        "version": "0.0",
        "apiVersion": 1,
        "lang": lang,
        "baseUrl": base_url,
        "authors": author_list,
        "isDeprecated": False,
        "iconUrl": f"{base_url}/favicon.ico",
        "webviewNeeded": False,
        "runnerConcurrency": 3,
        "runnerCooldown": 1000,
        "maxAttempts": 3,
    }
    manifest_path = target_dir / "manifest.json"
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2)
        f.write("\n")

    # 3. Generate CHANGELOG.md
    today = datetime.date.today().isoformat()
    changelog_content = f"""# Changelog - {name}

## [0.0] - {today}
- Initial release.
"""
    changelog_path = target_dir / "CHANGELOG.md"
    changelog_path.write_text(changelog_content, encoding="utf-8")

    # 3. Generate Cargo.toml & src/lib.rs
    if template:
        template_id = sanitize_id(template)
        template_crate = f"{template_id}-template"
        engine_struct = f"{to_pascal_case(template_id)}Engine"

        cargo_toml = f"""[package]
name = "{ext_id}"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
bunori-sdk.workspace = true
{template_crate} = {{ path = "../../templates/{template_id}" }}
"""
        lib_rs = f"""use bunori_sdk::*;
use {template_crate.replace('-', '_')}::{engine_struct};

#[derive(Default)]
pub struct {pascal_name}Source;

impl {pascal_name}Source {{
    fn engine(&self) -> {engine_struct} {{
        let meta = self.metadata();
        {engine_struct}::new(meta.base_url)
    }}
}}

impl Source for {pascal_name}Source {{
    fn metadata(&self) -> SourceMetadata {{
        serde_json::from_str(include_str!("../manifest.json")).expect("Invalid manifest.json")
    }}

    fn search(&self, query: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {{
        self.engine().search(query, page)
    }}

    fn get_novel_details(&self, novel_url: &str) -> Result<NovelDto, String> {{
        self.engine().get_novel_details(novel_url)
    }}

    fn get_chapter_content(&self, chapter_url: &str) -> Result<Option<String>, String> {{
        self.engine().get_chapter_content(chapter_url)
    }}

    fn get_listings(&self) -> Vec<ListingDto> {{
        self.engine().get_listings()
    }}

    fn get_listing_novels(&self, listing_id: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {{
        self.engine().get_listing_novels(listing_id, page)
    }}
}}

export_source!({pascal_name}Source);
"""
    else:
        cargo_toml = f"""[package]
name = "{ext_id}"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
bunori-sdk.workspace = true
"""
        lib_template = """use bunori_sdk::*;
use std::collections::HashMap;

#[derive(Default)]
pub struct __PASCAL_NAME__Source;

impl Source for __PASCAL_NAME__Source {
    fn metadata(&self) -> SourceMetadata {
        serde_json::from_str(include_str!("../manifest.json"))
            .expect("Invalid manifest.json")
    }

    fn search(&self, query: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {
        let meta = self.metadata();
        let formatted = query.replace(' ', "+");
        let search_url = format!("{}/search?keyword={}&page={}", meta.base_url, formatted, page);
        let doc = host::document(&search_url, None)?;

        // TODO: Replace with the website's CSS selectors
        let item_sel = Selector::parse(".search-item").map_err(|e| e.to_string())?;
        let title_sel = Selector::parse("h3.title a").map_err(|e| e.to_string())?;
        let cover_sel = Selector::parse("img.cover").map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for el in doc.select(&item_sel) {
            let Some(link) = el.select(&title_sel).next() else { continue; };
            let title = link.text().collect::<Vec<_>>().join("").trim().to_string();
            let Some(href) = link.value().attr("href") else { continue; };
            let url = if href.starts_with("http") { href.to_string() } else { format!("{}{}", meta.base_url, href) };

            let cover_url = el.select(&cover_sel).next()
                .and_then(|img| img.value().attr("src"))
                .map(|s| if s.starts_with("http") { s.to_string() } else { format!("{}{}", meta.base_url, s) });

            if !title.is_empty() && !url.is_empty() {
                results.push(SearchResultDto {
                    url,
                    title,
                    cover_url,
                    author: None,
                });
            }
        }

        Ok(results)
    }

    fn get_novel_details(&self, novel_url: &str) -> Result<NovelDto, String> {
        let meta = self.metadata();
        let doc = host::document(novel_url, None)?;

        // TODO: Replace with novel detail selectors
        let title_sel = Selector::parse("h1.title").map_err(|e| e.to_string())?;
        let desc_sel = Selector::parse(".description").map_err(|e| e.to_string())?;
        let cover_sel = Selector::parse(".cover img").map_err(|e| e.to_string())?;

        let title = doc.select(&title_sel).next()
            .map(|t| t.text().collect::<Vec<_>>().join("").trim().to_string())
            .unwrap_or_default();

        let cover_url = doc.select(&cover_sel).next()
            .and_then(|img| img.value().attr("src"))
            .map(|s| if s.starts_with("http") { s.to_string() } else { format!("{}{}", meta.base_url, s) });

        let description = doc.select(&desc_sel).next()
            .map(|d| d.text().collect::<Vec<_>>().join("").trim().to_string());

        // TODO: Parse chapters
        let mut chapters = Vec::new();
        if let Ok(ch_sel) = Selector::parse("ul.chapters li a") {
            for (i, el) in doc.select(&ch_sel).enumerate() {
                let Some(href) = el.value().attr("href") else { continue; };
                let ch_url = if href.starts_with("http") { href.to_string() } else { format!("{}{}", meta.base_url, href) };
                let ch_title = el.text().collect::<Vec<_>>().join("").trim().to_string();

                chapters.push(ChapterDto {
                    url: ch_url,
                    title: if ch_title.is_empty() { format!("Chapter {}", i + 1) } else { ch_title },
                    index: (i + 1) as i32,
                    release_date: None,
                    scanlation: None,
                });
            }
        }

        Ok(NovelDto {
            url: novel_url.to_string(),
            title,
            author: None,
            cover_url,
            description,
            status: None,
            genres: Vec::new(),
            chapters,
            extra: HashMap::new(),
        })
    }

    fn get_chapter_content(&self, chapter_url: &str) -> Result<Option<String>, String> {
        let doc = host::document(chapter_url, None)?;

        // TODO: Replace with chapter content selector
        let Ok(sel) = Selector::parse("#chapter-content, .chapter-text") else {
            return Ok(None);
        };

        let Some(el) = doc.select(&sel).next() else {
            return Ok(None);
        };

        let mut content = el.inner_html();
        // Strip unwanted tags
        for rem in &["script", "style", "ins"] {
            let open = format!("<{}", rem);
            let close = format!("</{}>", rem);
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
}

export_source!(__PASCAL_NAME__Source);
"""
        lib_rs = lib_template.replace("__PASCAL_NAME__", pascal_name)

    cargo_path = target_dir / "Cargo.toml"
    with open(cargo_path, "w", encoding="utf-8") as f:
        f.write(cargo_toml)

    lib_path = src_dir / "lib.rs"
    with open(lib_path, "w", encoding="utf-8") as f:
        f.write(lib_rs)

    # 4. Fetch & optimize website icon (WebP)
    try:
        from fetch_icons import fetch_and_save_icon
        fetch_and_save_icon(target_dir)
    except Exception as e:
        print(f"  ⚠ Could not auto-fetch icon: {e}")

    print(f"\n✨ Successfully created new source extension: {name} ({ext_id})")
    print(f"  📁 Location:   sources/{ext_id}/")
    print(f"  📄 Manifest:   sources/{ext_id}/manifest.json")
    print(f"  📝 Changelog:  sources/{ext_id}/CHANGELOG.md")
    print(f"  ⚙ Cargo:      sources/{ext_id}/Cargo.toml")
    print(f"  🦀 Code:       sources/{ext_id}/src/lib.rs")
    if template:
        print(f"  🧩 Template:   templates/{template}/")
    print("\nNext steps:")
    print(f"  1. Test compilation: cargo check --package {ext_id}")
    print(f"  2. Test host:        python tools/test.py {ext_id} --metadata")
    print(f"  3. Package it:       python tools/package.py --single {ext_id}\n")


def main():
    parser = argparse.ArgumentParser(description="Create a new Bunori source extension skeleton.")
    parser.add_argument("-n", "--name", help="Human-readable extension name (e.g. 'Novel Hi')")
    parser.add_argument("-u", "--url", help="Base URL for the website (e.g. 'https://novelhi.com')")
    parser.add_argument("-i", "--id", help="Extension identifier (optional, default: derived from name)")
    parser.add_argument("-a", "--author", "--authors", dest="authors", help="Author handle(s) (comma-separated, e.g. 'zenit')")
    parser.add_argument("-l", "--lang", default="en", help="Language code (default: 'en')")
    parser.add_argument("-t", "--template", help="Template engine crate to use (e.g. 'madara')")

    args = parser.parse_args()

    name = args.name
    if not name:
        try:
            name = input("Extension Name (e.g. Novel Hi): ").strip()
        except (KeyboardInterrupt, EOFError):
            print("\nCancelled.")
            sys.exit(0)

    if not name:
        print("Error: Extension name cannot be empty.", file=sys.stderr)
        sys.exit(1)

    url = args.url
    if not url:
        try:
            url = input("Base URL (e.g. https://novelhi.com): ").strip()
        except (KeyboardInterrupt, EOFError):
            print("\nCancelled.")
            sys.exit(0)

    if not url:
        print("Error: Base URL cannot be empty.", file=sys.stderr)
        sys.exit(1)

    authors = args.authors
    if not authors and sys.stdin.isatty():
        default_author = get_default_author()
        prompt_text = f"Author(s) [{default_author}]: " if default_author else "Author(s) (e.g. zenit): "
        try:
            val = input(prompt_text).strip()
            authors = val if val else (default_author or None)
        except (KeyboardInterrupt, EOFError):
            authors = default_author or None
    elif not authors:
        authors = get_default_author() or None

    create_source(
        name=name,
        base_url=url,
        ext_id=args.id,
        lang=args.lang,
        authors=authors,
        template=args.template,
    )


if __name__ == "__main__":
    main()
