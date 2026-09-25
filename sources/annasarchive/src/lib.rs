use bunori_sdk::*;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct AnnaSArchiveSource;

impl Source for AnnaSArchiveSource {
    fn metadata(&self) -> SourceMetadata {
        serde_json::from_str(include_str!("../manifest.json"))
            .expect("Invalid manifest.json")
    }

    fn search(&self, query: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {
        let meta = self.metadata();
        let formatted = query.replace(' ', "+");
        // Always request EPUBs so results are reflowable e-books
        let search_url = format!("{}/search?q={}&ext=epub&page={}", meta.base_url, formatted, page);
        let doc = host::document(&search_url, None)?;

        let link_sel = Selector::parse("a[href*='/md5/']").map_err(|e| e.to_string())?;
        let title_sel = Selector::parse("h3").map_err(|e| e.to_string())?;
        let img_sel = Selector::parse("img").map_err(|e| e.to_string())?;
        let author_sel = Selector::parse(".italic, div.truncate").map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        let mut seen_hashes = HashSet::new();

        for el in doc.select(&link_sel) {
            let Some(href) = el.value().attr("href") else { continue; };
            if !href.contains("/md5/") { continue; }

            // Extract the unique MD5 hash from the URL
            let hash = href.split("/md5/").nth(1).unwrap_or("").split(['?', '/']).next().unwrap_or("");
            if hash.is_empty() || !seen_hashes.insert(hash.to_string()) {
                continue;
            }

            let url = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("{}{}", meta.base_url, href)
            };

            // Title: check h3 inside link, or fallback to element text
            let title = el.select(&title_sel).next()
                .map(|h| h.text().collect::<Vec<_>>().join("").trim().to_string())
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| {
                    el.text().collect::<Vec<_>>().join(" ").trim().to_string()
                });

            if title.is_empty() { continue; }

            // Cover image
            let cover_url = el.select(&img_sel).next()
                .and_then(|img| img.value().attr("src"))
                .map(|s| {
                    if s.starts_with("http") {
                        s.to_string()
                    } else {
                        format!("{}{}", meta.base_url, s)
                    }
                });

            // Author
            let author = el.select(&author_sel).next()
                .map(|a| a.text().collect::<Vec<_>>().join("").trim().to_string())
                .filter(|a| !a.is_empty());

            results.push(SearchResultDto {
                url,
                title,
                cover_url,
                author,
            });
        }

        Ok(results)
    }

    fn get_novel_details(&self, novel_url: &str) -> Result<NovelDto, String> {
        let meta = self.metadata();
        let doc = host::document(novel_url, None)?;

        let hash = novel_url.split("/md5/").nth(1).unwrap_or("").split(['?', '/']).next().unwrap_or("unknown");
        let book_id = format!("aa_{}", hash);

        // 1. Title
        let h1_sel = Selector::parse("h1").map_err(|e| e.to_string())?;
        let title = doc.select(&h1_sel).next()
            .map(|t| t.text().collect::<Vec<_>>().join("").trim().to_string())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| format!("Book {}", hash));

        // 2. Author
        let author_sel = Selector::parse("div.italic, .author, span.italic").map_err(|e| e.to_string())?;
        let author = doc.select(&author_sel).next()
            .map(|a| a.text().collect::<Vec<_>>().join("").trim().to_string())
            .filter(|a| !a.is_empty());

        // 3. Cover Image
        let cover_sel = Selector::parse("img[src*='/covers/'], div.cover img, img").map_err(|e| e.to_string())?;
        let cover_url = doc.select(&cover_sel).next()
            .and_then(|img| img.value().attr("src"))
            .map(|s| {
                if s.starts_with("http") {
                    s.to_string()
                } else {
                    format!("{}{}", meta.base_url, s)
                }
            });

        // 4. Description / Synopsis
        let desc_sel = Selector::parse(".js-md5-top-box-description, div.mb-4.text-base, .description").map_err(|e| e.to_string())?;
        let description = doc.select(&desc_sel).next()
            .map(|d| d.text().collect::<Vec<_>>().join("\n").trim().to_string())
            .filter(|d| !d.is_empty());

        // 5. Scrape Download Mirrors
        let a_sel = Selector::parse("a[href]").map_err(|e| e.to_string())?;
        let mut mirrors = Vec::new();
        let mut seen_mirrors = HashSet::new();

        for a in doc.select(&a_sel) {
            let Some(href) = a.value().attr("href") else { continue; };
            let is_mirror = href.contains("ipfs") 
                || href.contains("libgen") 
                || href.contains("/slow_download/") 
                || href.contains("/fast_download/")
                || href.contains("cloudflare-ipfs.com")
                || href.contains("gateway.pinata.cloud");

            if is_mirror {
                let full_url = if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("{}{}", meta.base_url, href)
                };

                if seen_mirrors.insert(full_url.clone()) {
                    mirrors.push(full_url);
                }
            }
        }

        let mut extra = HashMap::new();
        extra.insert("bookId".to_string(), book_id);
        if !mirrors.is_empty() {
            extra.insert("mirrors".to_string(), mirrors.join(";"));
        }

        Ok(NovelDto {
            url: novel_url.to_string(),
            title,
            author,
            cover_url,
            description,
            status: Some("Completed".to_string()),
            genres: vec!["E-Book".to_string()],
            chapters: Vec::new(), // Will be populated by native EpubBookManager via extra["mirrors"]
            extra,
        })
    }

    fn get_chapter_content(&self, _chapter_url: &str) -> Result<Option<String>, String> {
        // Native Bunori EpubBookManager intercepts epub:// chapter URLs directly
        Ok(None)
    }
}

export_source!(AnnaSArchiveSource);
