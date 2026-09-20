use bunori_sdk::*;
use std::collections::HashMap;

pub struct ReadNovelFullEngine {
    pub base_url: String,
    pub search_path: String,
    pub chapter_endpoint: String,

    // Selectors
    pub list_row_selector: String,
    pub list_title_selector: String,
    pub list_cover_selector: String,
    pub list_author_selector: String,

    pub detail_title_selector: String,
    pub detail_author_selector: String,
    pub detail_cover_selector: String,
    pub detail_desc_selector: String,
    pub detail_genre_selector: String,
    pub detail_status_selector: String,

    pub chapter_content_selector: String,
    pub listings: Vec<ListingDto>,
}

impl ReadNovelFullEngine {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            search_path: "search".into(),
            chapter_endpoint: "ajax-chapter-option".into(),

            list_row_selector: ".list-novel .row, .list-truyen .row, .rank-list .li-row, .li-row".into(),
            list_title_selector: "h3.novel-title a, h3.tit a, h3 a".into(),
            list_cover_selector: "img.cover, .pic img, img".into(),
            list_author_selector: "a[href*='/author/'], a[href*='/authors/'], .author".into(),

            detail_title_selector: "h3.title, h1.tit, .truyen-title, h1".into(),
            detail_author_selector: "a[href*='/author/'], a[href*='/authors/']".into(),
            detail_cover_selector: ".book img, .pic img, .cover img, img".into(),
            detail_desc_selector: ".desc-text, #novel-summary-inner, .inner, .txt".into(),
            detail_genre_selector: "a[href*='/genre/'], a[href*='/genres/']".into(),
            detail_status_selector: ".info-meta li, .m-imgtxt .item".into(),

            chapter_content_selector: "#chapter-content, #chr-content, .chapter-c, .txt".into(),

            listings: vec![
                ListingDto { id: "latest-release-novel".into(), name: "Latest Release".into() },
                ListingDto { id: "hot-novel".into(), name: "Hot Novels".into() },
                ListingDto { id: "completed-novel".into(), name: "Completed Novels".into() },
            ],
        }
    }

    pub fn parse_novel_list(&self, doc: &Html) -> Vec<SearchResultDto> {
        let Ok(row_sel) = Selector::parse(&self.list_row_selector) else { return Vec::new(); };
        let Ok(title_sel) = Selector::parse(&self.list_title_selector) else { return Vec::new(); };
        let Ok(cover_sel) = Selector::parse(&self.list_cover_selector) else { return Vec::new(); };
        let Ok(author_sel) = Selector::parse(&self.list_author_selector) else { return Vec::new(); };

        let mut results = Vec::new();
        for row in doc.select(&row_sel) {
            let Some(link) = row.select(&title_sel).next() else { continue; };
            let title = link.text().collect::<Vec<_>>().join("").trim().to_string();
            let Some(href) = link.value().attr("href") else { continue; };
            let url = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("{}{}", self.base_url, href)
            };

            let cover_url = row.select(&cover_sel).next()
                .and_then(|img| img.value().attr("src").or_else(|| img.value().attr("data-src")))
                .map(|s| if s.starts_with("http") { s.to_string() } else { format!("{}{}", self.base_url, s) });

            let author = row.select(&author_sel).next()
                .map(|a| a.text().collect::<Vec<_>>().join("").trim().to_string())
                .filter(|s| !s.is_empty());

            if !title.is_empty() && !url.is_empty() {
                results.push(SearchResultDto {
                    url,
                    title,
                    cover_url,
                    author,
                });
            }
        }

        results
    }

    pub fn search(&self, query: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {
        let formatted = query.replace(' ', "+");
        let search_url = format!("{}/{}?keyword={}&page={}", self.base_url, self.search_path, formatted, page);
        let doc = host::document(&search_url, None)?;
        Ok(self.parse_novel_list(&doc))
    }

    pub fn get_novel_details(&self, novel_url: &str) -> Result<NovelDto, String> {
        let doc = host::document(novel_url, None)?;

        let title_sel = Selector::parse(&self.detail_title_selector).map_err(|e| e.to_string())?;
        let author_sel = Selector::parse(&self.detail_author_selector).map_err(|e| e.to_string())?;
        let cover_sel = Selector::parse(&self.detail_cover_selector).map_err(|e| e.to_string())?;
        let desc_sel = Selector::parse(&self.detail_desc_selector).map_err(|e| e.to_string())?;
        let genre_sel = Selector::parse(&self.detail_genre_selector).map_err(|e| e.to_string())?;
        let item_sel = Selector::parse(&self.detail_status_selector).map_err(|e| e.to_string())?;

        let title = doc.select(&title_sel).next()
            .map(|t| t.text().collect::<Vec<_>>().join("").trim().to_string())
            .unwrap_or_default();

        let author = doc.select(&author_sel).next()
            .map(|a| a.text().collect::<Vec<_>>().join("").trim().to_string())
            .filter(|s| !s.is_empty());

        let cover_url = doc.select(&cover_sel).next()
            .and_then(|img| img.value().attr("src").or_else(|| img.value().attr("data-src")))
            .map(|s| if s.starts_with("http") { s.to_string() } else { format!("{}{}", self.base_url, s) });

        let description = doc.select(&desc_sel).next().map(|el| {
            if let Ok(p_sel) = Selector::parse("p") {
                let paragraphs: Vec<String> = el.select(&p_sel)
                    .map(|p| p.text().collect::<Vec<_>>().join("").trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !paragraphs.is_empty() {
                    return paragraphs.join("\n\n");
                }
            }
            el.text().collect::<Vec<_>>().join("").trim().to_string()
        });

        let mut genres = Vec::new();
        for el in doc.select(&genre_sel) {
            let g = el.text().collect::<Vec<_>>().join("").trim().to_string();
            if !g.is_empty() && !genres.contains(&g) {
                genres.push(g);
            }
        }

        let mut status = None;
        let time_sel = Selector::parse("span[title='Status'], .glyphicon-time").ok();
        let right_sel = Selector::parse(".right, a, span").ok();
        for item in doc.select(&item_sel) {
            if let Some(ref ts) = time_sel {
                if item.select(ts).next().is_some() {
                    if let Some(ref rs) = right_sel {
                        if let Some(r) = item.select(rs).next() {
                            let text = r.text().collect::<Vec<_>>().join("").trim().to_string();
                            if !text.is_empty() {
                                status = Some(text);
                                break;
                            }
                        }
                    }
                }
            }
        }

        // Novel ID for AJAX chapters
        let mut novel_id = String::new();
        if let Ok(id_sel) = Selector::parse("[data-novel-id], input#truyen-id") {
            if let Some(el) = doc.select(&id_sel).next() {
                if let Some(id) = el.value().attr("data-novel-id").or_else(|| el.value().attr("value")) {
                    novel_id = id.trim().to_string();
                }
            }
        }
        if novel_id.is_empty() {
            let html = doc.html();
            for pattern in &["novelId =", "novelId=", "truyenId =", "truyenId="] {
                if let Some(idx) = html.find(pattern) {
                    let digits: String = html[idx + pattern.len()..]
                        .chars()
                        .filter(|c| *c != '"' && *c != '\'')
                        .skip_while(|c| c.is_whitespace())
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    if !digits.is_empty() {
                        novel_id = digits;
                        break;
                    }
                }
            }
        }

        let mut chapters = Vec::new();
        if !novel_id.is_empty() {
            let ep = format!("{}/{}?novelId={}", self.base_url, self.chapter_endpoint, novel_id);
            let mut hdrs = HashMap::new();
            hdrs.insert("Referer".to_string(), novel_url.to_string());
            hdrs.insert("X-Requested-With".to_string(), "XMLHttpRequest".to_string());

            if let Ok(ajax_html) = host::get(&ep, Some(hdrs)) {
                let ajax_doc = Html::parse_fragment(&ajax_html);
                // 1. Dropdown <select><option value="...">Chapter 1</option>
                if let Ok(opt_sel) = Selector::parse("option") {
                    let options: Vec<_> = ajax_doc.select(&opt_sel).collect();
                    if options.len() > 1 {
                        for (i, opt) in options.into_iter().enumerate() {
                            let Some(val) = opt.value().attr("value") else { continue; };
                            let val = val.trim();
                            if val.is_empty() || val == "0" { continue; }
                            let ch_url = if val.starts_with("http") {
                                val.to_string()
                            } else {
                                format!("{}{}", self.base_url, val)
                            };
                            let ch_title = opt.text().collect::<Vec<_>>().join("").trim().to_string();
                            chapters.push(ChapterDto {
                                url: ch_url,
                                title: if ch_title.is_empty() { format!("Chapter {}", i + 1) } else { ch_title },
                                index: (i + 1) as i32,
                                release_date: None,
                                scanlation: None,
                            });
                        }
                    }
                }

                // 2. Link lists: <a>
                if chapters.is_empty() {
                    if let Ok(a_sel) = Selector::parse("a") {
                        for (i, a) in ajax_doc.select(&a_sel).enumerate() {
                            let Some(href) = a.value().attr("href") else { continue; };
                            let href = href.trim();
                            if href.is_empty() || href == "javascript:;" || href == "#" { continue; }
                            let ch_url = if href.starts_with("http") {
                                href.to_string()
                            } else {
                                format!("{}{}", self.base_url, href)
                            };
                            let ch_title = a.value().attr("title")
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|| a.text().collect::<Vec<_>>().join("").trim().to_string());
                            chapters.push(ChapterDto {
                                url: ch_url,
                                title: if ch_title.is_empty() { format!("Chapter {}", i + 1) } else { ch_title },
                                index: (i + 1) as i32,
                                release_date: None,
                                scanlation: None,
                            });
                        }
                    }
                }
            }
        }

        // Fallback: static table / list elements in page
        if chapters.is_empty() {
            if let Ok(row_sel) = Selector::parse("#idData li a, #list-chapter ul li a, ul.list-chapter li a") {
                for (i, el) in doc.select(&row_sel).enumerate() {
                    let Some(href) = el.value().attr("href") else { continue; };
                    let trimmed_href = href.trim();
                    if trimmed_href.is_empty() || trimmed_href == "javascript:;" || trimmed_href == "#" {
                        continue;
                    }
                    let ch_url = if trimmed_href.starts_with("http") {
                        trimmed_href.to_string()
                    } else {
                        format!("{}{}", self.base_url, trimmed_href)
                    };
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
        }

        Ok(NovelDto {
            url: novel_url.to_string(),
            title,
            author,
            cover_url,
            description,
            status,
            genres,
            chapters,
            extra: HashMap::new(),
        })
    }

    pub fn get_chapter_content(&self, chapter_url: &str) -> Result<Option<String>, String> {
        let mut hdrs = HashMap::new();
        hdrs.insert("Referer".to_string(), chapter_url.to_string());
        let html = host::get(chapter_url, Some(hdrs))?;
        let doc = Html::parse_document(&html);

        let sel = Selector::parse(&self.chapter_content_selector).map_err(|e| e.to_string())?;
        let Some(el) = doc.select(&sel).next() else {
            return Ok(None);
        };

        let mut content = el.inner_html();
        for rem in &["script", "style", "ins", "iframe", "button", "noscript"] {
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

    pub fn get_listings(&self) -> Vec<ListingDto> {
        self.listings.clone()
    }

    pub fn get_listing_novels(&self, listing_id: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {
        let url = format!("{}/{}?page={}", self.base_url, listing_id, page);
        let doc = host::document(&url, None)?;
        Ok(self.parse_novel_list(&doc))
    }
}
