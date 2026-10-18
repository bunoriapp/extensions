use bunori_sdk::*;
use std::collections::HashMap;

#[derive(Default)]
pub struct DaoteknoSource;

impl DaoteknoSource {
    fn default_headers() -> Option<HashMap<String, String>> {
        let mut hdrs = HashMap::new();
        hdrs.insert(
            "User-Agent".to_string(),
            "Mozilla/5.0 (Android; Mobile; rv:155.0) Gecko/155.0 Firefox/155.0".to_string(),
        );
        hdrs.insert(
            "Accept".to_string(),
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8".to_string(),
        );
        hdrs.insert("Sec-CH-UA-Mobile".to_string(), "?1".to_string());
        hdrs.insert("Sec-CH-UA-Platform".to_string(), "\"Android\"".to_string());
        Some(hdrs)
    }

    fn parse_cards(doc: &Html, base_url: &str) -> Result<Vec<SearchResultDto>, String> {
        let card_sel = Selector::parse("a.novel-card, a.novel-item").map_err(|e| e.to_string())?;
        let title_sel = Selector::parse(".novel-title").map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for card in doc.select(&card_sel) {
            let Some(href) = card.value().attr("href") else { continue; };
            if href.is_empty() || href == "/" {
                continue;
            }

            let full_url = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("{}{}", base_url.trim_end_matches('/'), href)
            };

            let title = if let Some(t_el) = card.select(&title_sel).next() {
                t_el.text().collect::<Vec<_>>().join("").trim().to_string()
            } else {
                card.text().collect::<Vec<_>>().join("").trim().to_string()
            };

            if !title.is_empty() && !results.iter().any(|r: &SearchResultDto| r.url == full_url) {
                results.push(SearchResultDto {
                    url: full_url,
                    title,
                    cover_url: None,
                    author: None,
                });
            }
        }

        Ok(results)
    }
}

impl Source for DaoteknoSource {
    fn metadata(&self) -> SourceMetadata {
        serde_json::from_str(include_str!("../manifest.json"))
            .expect("Invalid manifest.json")
    }

    fn search(&self, query: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {
        let meta = self.metadata();
        let base = meta.base_url.trim_end_matches('/');
        let formatted = query.replace(' ', "+");
        let search_url = if page > 1 {
            format!("{}/?q={}&page={}", base, formatted, page)
        } else {
            format!("{}/?q={}", base, formatted)
        };

        let doc = host::document(&search_url, Self::default_headers())?;
        Self::parse_cards(&doc, &meta.base_url)
    }

    fn get_novel_details(&self, novel_url: &str) -> Result<NovelDto, String> {
        let meta = self.metadata();
        let base = meta.base_url.trim_end_matches('/');

        let doc_first = host::document(novel_url, Self::default_headers())?;

        let title_sel = Selector::parse("h1").map_err(|e| e.to_string())?;
        let title = doc_first
            .select(&title_sel)
            .next()
            .map(|t| t.text().collect::<Vec<_>>().join("").trim().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let ch_sel = Selector::parse("a.chapter-box").map_err(|e| e.to_string())?;
        let next_sel = Selector::parse("div.pagination a.btn-page, a.btn-page")
            .map_err(|e| e.to_string())?;

        let mut raw_chapters = Vec::new();
        let mut current_page = 1;

        loop {
            let doc = if current_page == 1 {
                doc_first.clone()
            } else {
                let page_url = format!("{}/?page={}", novel_url.trim_end_matches('/'), current_page);
                host::document(&page_url, Self::default_headers())?
            };

            let mut page_items = Vec::new();
            for el in doc.select(&ch_sel) {
                // Skip newest chapters preview on top to avoid duplicates
                if el.value().classes().any(|c| c == "latest") {
                    continue;
                }

                let Some(href) = el.value().attr("href") else { continue; };
                let full_url = if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("{}{}", base, href)
                };

                let raw_text = el.text().collect::<Vec<_>>().join("").trim().to_string();
                let ch_title = if raw_text.is_empty() {
                    let slug_num = href.trim_matches('/').rsplit('/').next().unwrap_or("");
                    format!("Chapter {}", slug_num)
                } else if raw_text.chars().all(|c| c.is_ascii_digit()) {
                    format!("Chapter {}", raw_text)
                } else if !raw_text.to_lowercase().starts_with("chapter") {
                    format!("Chapter {}", raw_text)
                } else {
                    raw_text
                };

                page_items.push((full_url, ch_title));
            }

            if page_items.is_empty() {
                break;
            }

            raw_chapters.extend(page_items);

            let has_next = doc.select(&next_sel).any(|a| {
                let t = a.text().collect::<Vec<_>>().join("");
                t.contains("Next") || t.contains('»')
            });

            if has_next && current_page < 100 {
                current_page += 1;
            } else {
                break;
            }
        }

        // Chapters are listed newest first; reverse so Chapter 1 is index 1
        raw_chapters.reverse();

        let chapters: Vec<ChapterDto> = raw_chapters
            .into_iter()
            .enumerate()
            .map(|(i, (url, title))| ChapterDto {
                url,
                title,
                index: (i + 1) as i32,
                release_date: None,
                scanlation: None,
            })
            .collect();

        Ok(NovelDto {
            url: novel_url.to_string(),
            title,
            author: None,
            cover_url: None,
            description: None,
            status: None,
            genres: Vec::new(),
            chapters,
            extra: HashMap::new(),
        })
    }

    fn get_chapter_content(&self, chapter_url: &str) -> Result<Option<String>, String> {
        let doc = host::document(chapter_url, Self::default_headers())?;

        let Ok(sel) = Selector::parse("#novel-content") else {
            return Ok(None);
        };

        let Some(el) = doc.select(&sel).next() else {
            return Ok(None);
        };

        let mut content = el.inner_html();

        // Strip script, style, ins, iframe, button
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

        // Strip promo, donation, and captcha elements
        for cls in &["promo", "cbc-captcha-container", "pesan-donasi"] {
            let pat = format!("class=\"{}", cls);
            while let Some(pos) = content.find(&pat) {
                if let Some(s) = content[..pos].rfind("<div") {
                    if let Some(e) = content[pos..].find("</div>") {
                        content.replace_range(s..pos + e + 6, "");
                        continue;
                    }
                }
                break;
            }
        }

        Ok(Some(content.trim().to_string()))
    }

    fn get_listings(&self) -> Vec<ListingDto> {
        vec![ListingDto {
            id: "latest".to_string(),
            name: "All Novels".to_string(),
        }]
    }

    fn get_listing_novels(&self, _listing_id: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {
        let meta = self.metadata();
        let base = meta.base_url.trim_end_matches('/');
        let url = if page > 1 {
            format!("{}/?page={}", base, page)
        } else {
            format!("{}/", base)
        };

        let doc = host::document(&url, Self::default_headers())?;
        Self::parse_cards(&doc, &meta.base_url)
    }
}

export_source!(DaoteknoSource);
