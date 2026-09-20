"""
Scaffolding CLI for creating a new Bunori engine template.
Generates templates/<name>/Cargo.toml and templates/<name>/src/lib.rs.

Usage:
  python tools/template.py
  python tools/template.py -n madara
  python tools/template.py --name "Novel Fire" --struct-name NovelFireEngine
"""

import argparse
import re
import sys
from pathlib import Path


def sanitize_id(name: str) -> str:
    """Derive a valid Cargo crate / template ID from the name."""
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


def create_template(name: str, struct_name: str | None = None):
    project_root = Path(__file__).resolve().parent.parent
    templates_dir = project_root / "templates"

    template_id = sanitize_id(name)
    if not template_id:
        print("Error: Invalid template name provided.", file=sys.stderr)
        sys.exit(1)

    target_dir = templates_dir / template_id
    if target_dir.exists():
        print(f"Error: Template directory already exists at {target_dir}", file=sys.stderr)
        sys.exit(1)

    if not struct_name:
        struct_name = to_pascal_case(name)
        if not struct_name.endswith("Engine"):
            struct_name += "Engine"

    # 1. Create directories
    src_dir = target_dir / "src"
    src_dir.mkdir(parents=True, exist_ok=True)

    # 2. Generate Cargo.toml
    cargo_toml = f"""[package]
name = "{template_id}-template"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["rlib"]

[dependencies]
bunori-sdk.workspace = true
serde.workspace = true
serde_json.workspace = true
"""
    cargo_path = target_dir / "Cargo.toml"
    with open(cargo_path, "w", encoding="utf-8") as f:
        f.write(cargo_toml)

    # 3. Generate starter src/lib.rs
    lib_rs = f"""use bunori_sdk::*;
use std::collections::HashMap;

pub struct {struct_name} {{
    pub base_url: String,
    pub listings: Vec<ListingDto>,
}}

impl {struct_name} {{
    pub fn new(base_url: impl Into<String>) -> Self {{
        Self {{
            base_url: base_url.into(),
            listings: vec![
                ListingDto {{ id: "latest".into(), name: "Latest Release".into() }},
                ListingDto {{ id: "popular".into(), name: "Popular".into() }},
            ],
        }}
    }}

    pub fn search(&self, query: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {{
        // TODO: Implement template search parsing logic
        Ok(Vec::new())
    }}

    pub fn get_novel_details(&self, novel_url: &str) -> Result<NovelDto, String> {{
        // TODO: Implement template novel details parsing logic
        Ok(NovelDto {{
            url: novel_url.to_string(),
            title: "Untitled".into(),
            author: None,
            cover_url: None,
            description: None,
            status: None,
            genres: Vec::new(),
            chapters: Vec::new(),
            extra: HashMap::new(),
        }})
    }}

    pub fn get_chapter_content(&self, chapter_url: &str) -> Result<Option<String>, String> {{
        // TODO: Implement template chapter content parsing logic
        Ok(None)
    }}

    pub fn get_listings(&self) -> Vec<ListingDto> {{
        self.listings.clone()
    }}

    pub fn get_listing_novels(&self, listing_id: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {{
        // TODO: Implement template listing novels parsing logic
        Ok(Vec::new())
    }}
}}
"""
    lib_path = src_dir / "lib.rs"
    with open(lib_path, "w", encoding="utf-8") as f:
        f.write(lib_rs)

    print(f"\n✨ Successfully created new template: {name} ({template_id}-template)")
    print(f"  📁 Location:      templates/{template_id}/")
    print(f"  ⚙ Cargo:         templates/{template_id}/Cargo.toml")
    print(f"  🦀 Engine Code:   templates/{template_id}/src/lib.rs")
    print(f"  🏛 Struct Name:   {struct_name}")
    print("\nNext steps:")
    print(f"  1. Implement parsing methods in templates/{template_id}/src/lib.rs.")
    print(f"  2. Create a source using this template:")
    print(f"     python tools/new_source.py -n 'Example Site' -u 'https://example.com' --template {template_id}\n")


def main():
    parser = argparse.ArgumentParser(description="Create a new Bunori template engine skeleton.")
    parser.add_argument("-n", "--name", help="Template name (e.g. 'madara', 'novelfire')")
    parser.add_argument("-s", "--struct-name", help="Rust struct engine name (optional, default: NameEngine)")

    args = parser.parse_args()

    name = args.name
    if not name:
        try:
            name = input("Template Name (e.g. madara): ").strip()
        except (KeyboardInterrupt, EOFError):
            print("\nCancelled.")
            sys.exit(0)

    if not name:
        print("Error: Template name cannot be empty.", file=sys.stderr)
        sys.exit(1)

    create_template(name=name, struct_name=args.struct_name)


if __name__ == "__main__":
    main()
