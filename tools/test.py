import argparse
import json
import re
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

from curl_cffi import requests as curl_requests
from wasmtime import Engine, Func, FuncType, Linker, Memory, Module, Store, ValType


def detect_extension_user_agent(project_root: Path, ext_id: str) -> str:
    """Detect if an extension defines a specific User-Agent (such as mobile) in its source code."""
    src_file = project_root / "sources" / ext_id / "src" / "lib.rs"
    if src_file.exists():
        text = src_file.read_text(encoding="utf-8", errors="ignore")
        m = re.search(r"[\"']user-agent[\"'][\s\S]{1,60}?[\"'](Mozilla/[^\"']+)[\"']", text, re.IGNORECASE)
        if m:
            return m.group(1)
    return ""


def grab_cloudflare_cookies(url: str, user_agent: str = "", cookie_file: Path | None = None) -> tuple[str, str]:
    """Launch a browser window via Playwright to solve and grab Cloudflare cf_clearance cookie."""
    try:
        from playwright.sync_api import sync_playwright
    except ImportError:
        print("\n [ERROR] 'playwright' is required for --grab-cf. Run: pip install playwright && python -m playwright install chromium")
        return "", user_agent

    print(f"\n=======================================================")
    print(f" [CLOUDFLARE HELPER] Opening browser to grab clearance cookie...")
    print(f" Target URL: {url}")
    if user_agent:
        print(f" Emulating User-Agent: {user_agent}")
    print(f" Please complete any Turnstile / Captcha verification in the browser if prompted.")
    print(f" Listening for cf_clearance cookie (timeout: 120s)...")
    print(f"=======================================================\n")

    captured_cookie = ""
    captured_ua = user_agent

    with sync_playwright() as p:
        args = [
            "--disable-blink-features=AutomationControlled",
            "--no-sandbox",
        ]
        browser = p.chromium.launch(headless=False, args=args)

        context_kwargs = {}
        if user_agent:
            context_kwargs["user_agent"] = user_agent
            is_mobile = any(m in user_agent.lower() for m in ["mobile", "android", "iphone", "ipad"])
            if is_mobile:
                context_kwargs["viewport"] = {"width": 412, "height": 915}
                context_kwargs["is_mobile"] = True
                context_kwargs["has_touch"] = True

        context = browser.new_context(**context_kwargs)
        page = context.new_page()
        page.add_init_script("Object.defineProperty(navigator, 'webdriver', {get: () => undefined});")

        try:
            page.goto(url, timeout=60000, wait_until="commit")
        except Exception as e:
            print(f" [DEBUG] Navigation note: {e}")

        for _ in range(120):
            try:
                cookies = context.cookies()
                cf = next((c for c in cookies if c.get("name") == "cf_clearance"), None)
                if cf and cf.get("value"):
                    captured_cookie = f"cf_clearance={cf['value']}"
                    try:
                        eval_ua = page.evaluate("navigator.userAgent")
                        if eval_ua:
                            captured_ua = eval_ua
                    except Exception:
                        pass
                    print(f" [SUCCESS] Captured Cloudflare clearance cookie!")
                    break
            except Exception:
                pass
            time.sleep(1)

        browser.close()

    if captured_cookie:
        if cookie_file:
            try:
                cf_val = captured_cookie.split("=", 1)[1]
                with open(cookie_file, "w") as f:
                    f.write(f"cf_clearance {cf_val}\n")
                print(f" [SUCCESS] Saved cookie to {cookie_file.name}")
            except Exception as e:
                print(f" [WARN] Failed to write cookie to {cookie_file}: {e}")
    else:
        print(" [ERROR] Timed out waiting for Cloudflare clearance cookie.")

    return captured_cookie, captured_ua


class BunoriHostRunner:
    def __init__(
        self,
        wasm_path: Path,
        cookie: str = "",
        user_agent: str = "",
        auto_grab_cf: bool = False,
        default_cookie_file: Path | None = None,
        detected_ua: str = "",
    ):
        self.wasm_path = wasm_path
        self.cookie = cookie.strip()
        self.user_agent = user_agent.strip()
        self.auto_grab_cf = auto_grab_cf
        self.default_cookie_file = default_cookie_file
        self.detected_extension_ua = detected_ua
        self._cf_prompted = False

        self.engine = Engine()
        self.store = Store(self.engine)
        self.linker = Linker(self.engine)

        self._register_host_functions()

        if not wasm_path.exists():
            raise FileNotFoundError(f"WASM file not found at: {wasm_path}")

        self.module = Module.from_file(self.engine, str(wasm_path))
        self.instance = self.linker.instantiate(self.store, self.module)

        raw_memory = self.instance.exports(self.store)["memory"]
        assert isinstance(raw_memory, Memory), "Expected 'memory' export to be a Memory instance"
        self.memory = raw_memory

    def _get_func(self, name: str) -> Func:
        fn = self.instance.exports(self.store)[name]
        assert isinstance(fn, Func), f"Export '{name}' is not a Func"
        return fn

    def _register_host_functions(self):
        def host_time_ms(caller):
            return int(time.time() * 1000)

        def host_log(caller, level: int, ptr: int, length: int):
            mem = caller.get("memory")
            raw = mem.read(caller, ptr, ptr + length)
            msg = raw.decode("utf-8", errors="replace")
            levels = {1: "DEBUG", 2: "INFO", 3: "WARN", 4: "ERROR"}
            tag = levels.get(level, f"LOG_{level}")
            print(f" [{tag}] {msg}")

        def host_http(caller, req_ptr: int, req_len: int):
            mem = caller.get("memory")
            req_raw = mem.read(caller, req_ptr, req_ptr + req_len)
            req_data = json.loads(req_raw.decode("utf-8"))

            url = req_data["url"]
            method = req_data.get("method", "GET")
            headers = req_data.get("headers", {})
            body = req_data.get("body")

            ext_ua = headers.get("User-Agent") or headers.get("user-agent")
            if ext_ua and not self.detected_extension_ua:
                self.detected_extension_ua = ext_ua

            if self.user_agent:
                headers["User-Agent"] = self.user_agent
            elif "User-Agent" not in headers and "user-agent" not in headers:
                if self.detected_extension_ua:
                    headers["User-Agent"] = self.detected_extension_ua
                else:
                    headers["User-Agent"] = (
                        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/152.0.0.0 Safari/537.36"
                    )

            if self.cookie and "Cookie" not in headers and "cookie" not in headers:
                headers["Cookie"] = self.cookie

            data_bytes = body.encode("utf-8") if body else None

            def do_request():
                if curl_requests is not None:
                    try:
                        resp = curl_requests.request(
                            method=method,
                            url=url,
                            data=data_bytes,
                            headers=headers,
                            impersonate="chrome",
                            timeout=30,
                        )
                        return resp.status_code, resp.text, dict(resp.headers)
                    except Exception as e:
                        return 500, f"Network error: {e}", {}
                else:
                    http_req = urllib.request.Request(url, data=data_bytes, headers=headers, method=method)
                    try:
                        with urllib.request.urlopen(http_req, timeout=15) as resp:
                            return resp.status, resp.read().decode("utf-8", errors="replace"), dict(resp.headers)
                    except urllib.error.HTTPError as e:
                        return e.code, e.read().decode("utf-8", errors="replace"), dict(e.headers)
                    except Exception as e:
                        return 500, f"Network error: {e}", {}

            status_code, resp_body, resp_headers = do_request()

            # Check if Cloudflare 403 challenge was encountered
            is_cf_block = (
                status_code == 403
                and (
                    "just a moment" in resp_body.lower()
                    or "challenge" in resp_body.lower()
                    or "cf-mitigated" in str(resp_headers).lower()
                )
            )

            if is_cf_block and self.auto_grab_cf and not self._cf_prompted:
                self._cf_prompted = True
                target_ua = self.user_agent or headers.get("User-Agent") or self.detected_extension_ua
                new_cookie, new_ua = grab_cloudflare_cookies(
                    url=url,
                    user_agent=target_ua,
                    cookie_file=self.default_cookie_file,
                )
                if new_cookie:
                    self.cookie = new_cookie
                    headers["Cookie"] = new_cookie
                    if new_ua and not self.user_agent:
                        self.user_agent = new_ua
                        headers["User-Agent"] = new_ua
                    status_code, resp_body, resp_headers = do_request()

            res_payload = json.dumps({
                "status_code": status_code,
                "headers": resp_headers,
                "body": resp_body,
            }).encode("utf-8")

            alloc_func = caller.get("alloc")
            assert isinstance(alloc_func, Func), "Export 'alloc' not found on caller"
            res_ptr = alloc_func(caller, len(res_payload))
            assert isinstance(res_ptr, int)
            mem.write(caller, res_payload, res_ptr)

            return res_ptr | (len(res_payload) << 32)

        self.linker.define_func("bunori", "host_time_ms", FuncType([], [ValType.i64()]), host_time_ms, access_caller=True)
        self.linker.define_func("bunori", "host_log", FuncType([ValType.i32(), ValType.i32(), ValType.i32()], []), host_log, access_caller=True)
        self.linker.define_func("bunori", "host_http", FuncType([ValType.i32(), ValType.i32()], [ValType.i64()]), host_http, access_caller=True)

    def _write_string(self, text: str) -> tuple[int, int]:
        alloc_fun = self._get_func("alloc")
        payload = text.encode("utf-8")
        ptr = alloc_fun(self.store, len(payload))
        assert isinstance(ptr, int)
        self.memory.write(self.store, payload, ptr)
        return ptr, len(payload)

    def _read_and_free_string(self, packed_u64: int) -> str:
        if packed_u64 == 0:
            return ""
        ptr = packed_u64 & 0xFFFFFFFF
        length = (packed_u64 >> 32) & 0xFFFFFFFF
        raw = self.memory.read(self.store, ptr, ptr + length)
        result = raw.decode("utf-8", errors="replace")

        dealloc_item = self.instance.exports(self.store).get("dealloc")
        if isinstance(dealloc_item, Func):
            dealloc_item(self.store, ptr, length)

        return result

    def get_metadata(self) -> dict:
        fn = self._get_func("get_metadata")
        packed = fn(self.store)
        assert isinstance(packed, int)
        return json.loads(self._read_and_free_string(packed))

    def search(self, query: str, page: int = 1) -> list:
        q_ptr, q_len = self._write_string(query)
        fn = self._get_func("search")
        packed = fn(self.store, q_ptr, q_len, page)
        assert isinstance(packed, int)
        if packed == 0:
            raise RuntimeError(f"Search for '{query}' failed in extension (returned null/0). See [ERROR] logs above.")
        return json.loads(self._read_and_free_string(packed))

    def get_novel_details(self, url: str) -> dict:
        u_ptr, u_len = self._write_string(url)
        fn = self._get_func("get_novel_details")
        packed = fn(self.store, u_ptr, u_len)
        assert isinstance(packed, int)
        if packed == 0:
            raise RuntimeError(f"get_novel_details for '{url}' failed in extension (returned null/0). See [ERROR] logs above.")
        return json.loads(self._read_and_free_string(packed))

    def get_chapter_content(self, url: str) -> str:
        u_ptr, u_len = self._write_string(url)
        fn = self._get_func("get_chapter_content")
        packed = fn(self.store, u_ptr, u_len)
        assert isinstance(packed, int)
        if packed == 0:
            raise RuntimeError(f"get_chapter_content for '{url}' failed in extension (returned null/0). See [ERROR] logs above.")
        return self._read_and_free_string(packed)

    def get_listings(self) -> list:
        fn = self._get_func("get_listings")
        packed = fn(self.store)
        assert isinstance(packed, int)
        if packed == 0:
            raise RuntimeError("get_listings failed in extension (returned null/0). See [ERROR] logs above.")
        return json.loads(self._read_and_free_string(packed))

    def get_listing_novels(self, listing_id: str, page: int = 1) -> list:
        id_ptr, id_len = self._write_string(listing_id)
        fn = self._get_func("get_listing_novels")
        packed = fn(self.store, id_ptr, id_len, page)
        assert isinstance(packed, int)
        if packed == 0:
            raise RuntimeError(f"get_listing_novels for '{listing_id}' failed in extension (returned null/0). See [ERROR] logs above.")
        return json.loads(self._read_and_free_string(packed))


def parse_cookie_input(raw: str) -> str:
    if not raw:
        return ""
    raw = raw.strip()
    if not raw:
        return ""
    lines = [line.strip() for line in raw.splitlines() if line.strip() and not line.strip().startswith("#")]
    if not lines:
        return ""

    cookie_pairs = []
    is_multi_line_format = False
    for line in lines:
        parts = line.split(None, 1)
        if len(parts) >= 2 and "=" not in parts[0]:
            val_parts = parts[1].split()
            val = val_parts[0] if len(val_parts) > 1 and ("." in val_parts[1] or "/" in val_parts[1]) else parts[1].strip()
            cookie_pairs.append(f"{parts[0]}={val}")
            is_multi_line_format = True
        elif "=" in line:
            cookie_pairs.append(line.rstrip(";"))
            is_multi_line_format = True

    if is_multi_line_format and cookie_pairs:
        return "; ".join(cookie_pairs)

    return raw.replace("\n", " ").strip()


def main():
    parser = argparse.ArgumentParser(description="Test Bunori WASM extensions directly in python.")
    parser.add_argument("extension_id", help="Extension crate ID (for instance asianovel, novelarchive, daotekno)")
    parser.add_argument("--metadata", action="store_true", help="Fetch extension metadata")
    parser.add_argument("--search", help="Test search query")
    parser.add_argument("--page", type=int, default=1, help="Page number (default: 1)")
    parser.add_argument("--details", help="Test fetching novel details by URL")
    parser.add_argument("--chapter", help="Test fetching chapter content by URL")
    parser.add_argument("--listings", action="store_true", help="Fetch supported ranking listings")
    parser.add_argument("--listing-novels", help="Fetch novels for a listing ID")
    parser.add_argument("--wasm-path", help="Explicit path to .wasm binary")
    parser.add_argument("--cookie", "-c", default="", help="Cookie header string to pass to HTTP requests")
    parser.add_argument("--cookie-file", "-f", help="Path to file containing cookies")
    parser.add_argument("--user-agent", "-u", default="", help="Custom User-Agent string")
    parser.add_argument(
        "--grab-cf",
        action="store_true",
        help="Open a browser to solve and grab Cloudflare clearance cookies automatically",
    )
    args = parser.parse_args()

    project_root = Path(__file__).resolve().parent.parent
    tools_dir = Path(__file__).resolve().parent
    default_cookie_file = tools_dir / "cookies"

    cookie = ""
    if args.cookie_file:
        with open(args.cookie_file, "r") as f:
            cookie = parse_cookie_input(f.read())
    elif args.cookie:
        cookie = parse_cookie_input(args.cookie)
    elif default_cookie_file.exists():
        with open(default_cookie_file, "r") as f:
            cookie = parse_cookie_input(f.read())

    crate_name = args.extension_id.replace("-", "_")
    wasm_path = Path(args.wasm_path) if args.wasm_path else (
        project_root / "target" / "wasm32-unknown-unknown" / "release" / f"{crate_name}.wasm"
    )

    if not wasm_path.exists():
        print(f"Error: WASM file not found {wasm_path}")
        print(f"Please build it first with: cargo build -p {args.extension_id} --target wasm32-unknown-unknown --release")
        sys.exit(1)

    print(f"Loading WASM module: {wasm_path.name}")

    ext_detected_ua = detect_extension_user_agent(project_root, args.extension_id)
    active_ua = args.user_agent or ext_detected_ua

    # If --grab-cf was invoked standalone (no testing actions)
    has_test_action = any([args.search, args.details, args.chapter, args.listings, args.listing_novels])
    if args.grab_cf and not has_test_action:
        runner_temp = BunoriHostRunner(
            wasm_path,
            cookie=cookie,
            user_agent=active_ua,
            auto_grab_cf=False,
            default_cookie_file=default_cookie_file,
            detected_ua=ext_detected_ua,
        )
        meta = runner_temp.get_metadata()
        target_url = meta.get("baseUrl") or "https://daotekno.com"
        new_cookie, new_ua = grab_cloudflare_cookies(target_url, user_agent=active_ua, cookie_file=default_cookie_file)
        if new_cookie:
            print(f"\n[INFO] Cloudflare cookie successfully acquired and saved to {default_cookie_file.name}!")
            print("You can now run your tests (e.g. --search, --details).")
        sys.exit(0)

    # If --grab-cf was invoked alongside test actions but no cookie was provided
    if args.grab_cf and not cookie:
        runner_temp = BunoriHostRunner(
            wasm_path,
            cookie="",
            user_agent=active_ua,
            auto_grab_cf=False,
            default_cookie_file=default_cookie_file,
            detected_ua=ext_detected_ua,
        )
        meta = runner_temp.get_metadata()
        target_url = meta.get("baseUrl") or "https://daotekno.com"
        cookie, active_ua = grab_cloudflare_cookies(target_url, user_agent=active_ua, cookie_file=default_cookie_file)

    runner = BunoriHostRunner(
        wasm_path,
        cookie=cookie,
        user_agent=active_ua,
        auto_grab_cf=args.grab_cf,
        default_cookie_file=default_cookie_file,
        detected_ua=ext_detected_ua,
    )

    has_action = any([args.metadata, args.search, args.details, args.chapter, args.listings, args.listing_novels])

    if args.metadata or not has_action:
        print("\n--- [Metadata] ---")
        meta = runner.get_metadata()
        print(json.dumps(meta, indent=2))

    if args.search:
        print(f"\n--- [Search: '{args.search}' (page {args.page})] ---")
        results = runner.search(args.search, args.page)
        print(f"Found {len(results)} result(s):")
        print(json.dumps(results[:5], indent=2))
        if len(results) > 5:
            print(f"... and {len(results) - 5} more results.")

    if args.details:
        print(f"\n--- [Novel Details: '{args.details}'] ---")
        details = runner.get_novel_details(args.details)
        chapters = details.get("chapters", [])
        print(f"Title: {details.get('title')}")
        print(f"Author: {details.get('author')}")
        print(f"Genres: {', '.join(details.get('genres', []))}")
        print(f"Total Chapters: {len(chapters)}")
        if chapters:
            print(f"  First: {chapters[0].get('title')} -> {chapters[0].get('url')}")
            print(f"  Last:  {chapters[-1].get('title')} -> {chapters[-1].get('url')}")

    if args.chapter:
        print(f"\n--- [Chapter Content: '{args.chapter}'] ---")
        content = runner.get_chapter_content(args.chapter)
        print(f"Length: {len(content)} characters")
        print(f"Preview (first 400 chars):\n{content[:400]}...")

    if args.listings:
        print("\n--- [Supported Listings] ---")
        listings = runner.get_listings()
        print(json.dumps(listings, indent=2))

    if args.listing_novels:
        print(f"\n--- [Listing Novels: '{args.listing_novels}' (page {args.page})] ---")
        novels = runner.get_listing_novels(args.listing_novels, args.page)
        print(f"Found {len(novels)} novel(s):")
        print(json.dumps(novels[:5], indent=2))


if __name__ == "__main__":
    main()
