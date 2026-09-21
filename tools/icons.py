#!/usr/bin/env python3
"""
Bunori Extension Icon Fetcher & Optimizer
Fetches high-resolution website icons/favicons, resizes to 128x128 px, and converts to WebP.
"""

import io
import json
import re
import sys
from pathlib import Path
from urllib.parse import urljoin, urlparse

from curl_cffi import requests
from PIL import Image


def get_largest_frame(img: Image.Image) -> Image.Image:
    """If the image has multiple frames (e.g. .ico), return the frame with the largest dimensions."""
    best_frame = img
    max_area = img.width * img.height

    try:
        n_frames = getattr(img, "n_frames", 1)
        for i in range(n_frames):
            img.seek(i)
            frame = img.copy()
            area = frame.width * frame.height
            if area > max_area:
                max_area = area
                best_frame = frame
    except Exception:
        pass

    return best_frame


def find_icon_urls(html: str, base_url: str) -> list[str]:
    """Find icon URLs from HTML <link> tags, sorted by likely quality."""
    candidates = []

    # Regex search for <link ...>
    link_pattern = re.compile(r"<link\s+([^>]+)>", re.IGNORECASE)
    attr_pattern = re.compile(r'([a-zA-Z0-9_\-]+)=["\']([^"\']+)["\']')

    for match in link_pattern.finditer(html):
        attrs = dict(attr_pattern.findall(match.group(1)))
        rel = attrs.get("rel", "").lower().strip()
        href = attrs.get("href", "").strip()

        if not href:
            continue

        full_url = urljoin(base_url, href)

        if "apple-touch-icon" in rel:
            candidates.insert(0, (10, full_url))  # High priority
        elif "icon" in rel:
            sizes = attrs.get("sizes", "")
            priority = 5
            if "192" in sizes or "180" in sizes or "128" in sizes or "512" in sizes:
                priority = 9
            elif "96" in sizes or "64" in sizes or "48" in sizes or "32" in sizes:
                priority = 7
            candidates.append((priority, full_url))

    # Sort by priority descending
    candidates.sort(key=lambda x: x[0], reverse=True)
    return [url for _, url in candidates]


def optimize_icon(image_bytes: bytes, target_size: int = 128) -> bytes:
    """Converts image bytes to 128x128 RGBA WebP bytes."""
    with Image.open(io.BytesIO(image_bytes)) as img:
        img = get_largest_frame(img)
        img = img.convert("RGBA")

        w, h = img.size
        if w != h:
            # Fit inside target_size maintaining aspect ratio
            if w > h:
                new_w = target_size
                new_h = max(1, int(h * (target_size / w)))
            else:
                new_h = target_size
                new_w = max(1, int(w * (target_size / h)))

            resized = img.resize((new_w, new_h), Image.Resampling.LANCZOS)
            canvas = Image.new("RGBA", (target_size, target_size), (0, 0, 0, 0))
            offset = ((target_size - new_w) // 2, (target_size - new_h) // 2)
            canvas.paste(resized, offset, resized)
            final_img = canvas
        else:
            final_img = img.resize((target_size, target_size), Image.Resampling.LANCZOS)

        out = io.BytesIO()
        final_img.save(out, format="WEBP", quality=90, method=6)
        return out.getvalue()


def fetch_and_save_icon(source_dir: Path, target_size: int = 128) -> bool:
    manifest_file = source_dir / "manifest.json"
    if not manifest_file.exists():
        return False

    with open(manifest_file, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    ext_id = manifest.get("id", source_dir.name)
    base_url = manifest.get("baseUrl")
    icon_url = manifest.get("iconUrl")

    if not base_url and not icon_url:
        print(f"[{ext_id}] No baseUrl or iconUrl found.")
        return False

    session = requests.Session(impersonate="chrome")
    urls_to_try = []

    # 1. Inspect HTML on baseUrl
    if base_url:
        try:
            resp = session.get(base_url, timeout=10)
            if resp.status_code == 200:
                html_icons = find_icon_urls(resp.text, base_url)
                urls_to_try.extend(html_icons)
        except Exception as e:
            print(f"[{ext_id}] Warning fetching HTML from {base_url}: {e}")

    # 2. Manifest iconUrl
    if icon_url and icon_url not in urls_to_try:
        urls_to_try.append(icon_url)

    # 3. Common fallback favicon paths
    if base_url:
        for path in ["/favicon.ico", "/favicon.png", "/apple-touch-icon.png"]:
            fb_url = urljoin(base_url, path)
            if fb_url not in urls_to_try:
                urls_to_try.append(fb_url)

        # 4. Google favicon fallback service as safety net
        domain = urlparse(base_url).netloc
        g_url = f"https://t1.gstatic.com/faviconV2?client=SOCIAL&type=FAVICON&fallback_opts=TYPE,SIZE,URL&url=https://{domain}&size=128"
        urls_to_try.append(g_url)

    # Download and process the first working icon
    for url in urls_to_try:
        try:
            resp = session.get(url, timeout=10)
            if resp.status_code == 200 and resp.content and len(resp.content) > 30:
                webp_data = optimize_icon(resp.content, target_size=target_size)
                dest_file = source_dir / "icon.webp"
                dest_file.write_bytes(webp_data)
                size_kb = len(webp_data) / 1024
                print(f"✓ [{ext_id}] Saved {dest_file.name} ({size_kb:.2f} KB) from {url}")
                return True
        except Exception:
            continue

    print(f"✗ [{ext_id}] Failed to download icon from any candidate URL.")
    return False


def main():
    project_root = Path(__file__).resolve().parent.parent
    sources_dir = project_root / "sources"

    target = sys.argv[1] if len(sys.argv) > 1 else None

    if target:
        src = sources_dir / target
        if not src.exists():
            print(f"Source directory not found: {src}")
            sys.exit(1)
        sources = [src]
    else:
        sources = sorted([d for d in sources_dir.iterdir() if d.is_dir() and (d / "manifest.json").exists()])

    print(f"Fetching & optimizing icons for {len(sources)} extension(s)...")
    success = 0
    for src in sources:
        if fetch_and_save_icon(src):
            success += 1

    print(f"\nDone! Successfully processed {success}/{len(sources)} icons.")


if __name__ == "__main__":
    main()
