use bunori_sdk::*;
use std::collections::HashMap;

pub struct MadaraEngine {
    pub base_url: String,

    pub use_new_chapter_endpoint: bool,
    pub chapter_ajax_path: String,
    pub chapter_ajax_action: String,

    pub list_item_selector: String,
    pub list_title_selector: String,
    pub list_cover_selector: String,

    pub detail_title_selector: String,
    pub detail_author_selector: String,
    pub detail_cover_selector: String,
    pub detail_desc_selector: String,
    pub detail_genre_selector: String,
    pub detail_status_selector: String,

    pub chapter_item_selector: String,
    pub chapter_link_selector: String,
    pub chapter_date_selector: String,

    pub chapter_content_selector: String,

    pub listings: Vec<ListingDto>,
}

impl MadaraEngine {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),

            use_new_chapter_endpoint: false,
            chapter_ajax_path: "wp-admin/admin-ajax.php".into(),
            chapter_ajax_action: "manga_get_chapters".into(),

            list_item_selector: ".page-item-detail, .c-tabs-item__content, .manga-item, .post-item".into(),
            list_title_selector: ".post-title a, .manga-title a, h3 a, h4 a".into(),
            list_cover_selector: "img".into(),

            detail_title_selector: ".post-title h1, #manga-title h1, .manga-title, h1".into(),
            detail_author_selector: ".author-content a, .manga-author a, .manga-authors a, .author-content, .manga-author, .manga-authors".into(),
            detail_cover_selector: ".summary_image img, .manga-poster img, .post-thumbnail img, img".into(),
            detail_desc_selector: "div.summary__content, #tab-manga-about, .manga-summary, .manga-excerpt, .entry-content".into(),
            detail_genre_selector: ".genres-content a, .genre-content a, .post-content_item a[href*='/manga-genre/']".into(),
            detail_status_selector: ".post-status .summary-content, .manga-status, .post-content_item:contains('Status') .summary-content".into(),

            chapter_item_selector: ".wp-manga-chapter, li.wp-manga-chapter, .chapter-item".into(),
            chapter_link_selector: "a".into(),
            chapter_date_selector: "span.chapter-release-date, .chapter-release-date".into(),

            chapter_content_selector: ".text-left, .text-right, .entry-content, .c-blog-post > div > div:nth-child(2), .read-container, #chapter-ing, .reading-content".into(),

            listings: vec![
                ListingDto { id: "popular".into(), name: "Popular".into() },
                ListingDto { id: "latest".into(), name: "Latest Updates".into() },
                ListingDto { id: "new".into(), name: "New Novels".into() },
            ],
        }
    }

    fn abs_url(&self, href: &str) -> String {
        let href = href.trim();
        if href.starts_with("http://") || href.starts_with("https://") {
            href.to_string()
        } else {
            format!("{}/{}", self.base_url.trim_end_matches('/'), href.trim_start_matches('/'))
        }
    }

    pub fn parse_novel_list(&self, doc: &Html) -> Vec<SearchResultDto> {
        let Ok(item_sel) = Selector::parse(&self.list_item_selector) else { return Vec::new(); };
        let Ok(title_sel) = Selector::parse(&self.list_title_selector) else { return Vec::new(); };
        let Ok(cover_sel) = Selector::parse(&self.list_cover_selector) else { return Vec::new(); };

        let mut results = Vec::new();
        for item in doc.select(&item_sel) {
            let Some(link) = item.select(&title_sel).next() else { continue; };
            let title = link.text().collect::<Vec<_>>().join("").trim().to_string();
            let Some(href) = link.value().attr("href") else { continue; };
            if href.is_empty() || href == "#" { continue; }
            let url = self.abs_url(href);

            let cover_url = item.select(&cover_sel).next()
                .and_then(|img| {
                    img.value().attr("data-src")
                        .or_else(|| img.value().attr("src"))
                        .or_else(|| img.value().attr("data-lazy-src"))
                        .or_else(|| img.value().attr("data-lazy-srcset"))
                })
                .map(|s| self.abs_url(s));

            if !title.is_empty() && !url.is_empty() {
                results.push(SearchResultDto {
                    url,
                    title,
                    cover_url,
                    author: None,
                });
            }
        }

        results
    }

    pub fn search(&self, query: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {
        let formatted = query.replace(' ', "+");
        let search_url = format!(
            "{}/page/{}/?s={}&post_type=wp-manga",
            self.base_url.trim_end_matches('/'), page, formatted
        );
        let doc = host::document(&search_url, None)?;
        Ok(self.parse_novel_list(&doc))
    }

    pub fn get_novel_details(&self, novel_url: &str) -> Result<NovelDto, String> {
        let clean_url = novel_url.trim_end_matches('/');
        let doc = host::document(clean_url, None)?;

        let title_sel = Selector::parse(&self.detail_title_selector).map_err(|e| e.to_string())?;
        let cover_sel = Selector::parse(&self.detail_cover_selector).map_err(|e| e.to_string())?;

        let title = doc.select(&title_sel).next()
            .map(|t| t.text().collect::<Vec<_>>().join("").trim().to_string())
            .unwrap_or_default();

        let cover_url = doc.select(&cover_sel).next()
            .and_then(|img| {
                img.value().attr("data-lazy-src")
                    .or_else(|| img.value().attr("data-src"))
                    .or_else(|| img.value().attr("src"))
            })
            .map(|s| self.abs_url(s));

        // Author, Genres, Status, Description from items or fallback selectors
        let mut author = None;
        let mut status = None;
        let mut genres = Vec::new();

        if let Ok(item_sel) = Selector::parse(".post-content_item, .post-content") {
            if let Ok(h5_sel) = Selector::parse("h5") {
                if let Ok(content_sel) = Selector::parse(".summary-content, .summary_content") {
                    for item in doc.select(&item_sel) {
                        let label = item.select(&h5_sel).next()
                            .map(|h| h.text().collect::<Vec<_>>().join("").trim().to_lowercase())
                            .unwrap_or_default();

                        if let Some(content_el) = item.select(&content_sel).next() {
                            let text = content_el.text().collect::<Vec<_>>().join("").trim().to_string();
                            if label.contains("author") || label.contains("autor") || label.contains("المؤلف") {
                                if !text.is_empty() { author = Some(text); }
                            } else if label.contains("status") || label.contains("estado") || label.contains("durum") {
                                if !text.is_empty() { status = Some(text); }
                            } else if label.contains("genre") || label.contains("género") || label.contains("kategori") || label.contains("التصنيفات") {
                                if let Ok(a_sel) = Selector::parse("a") {
                                    for a in content_el.select(&a_sel) {
                                        let g = a.text().collect::<Vec<_>>().join("").trim().to_string();
                                        if !g.is_empty() && !genres.contains(&g) {
                                            genres.push(g);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback for Author if not found
        if author.is_none() {
            if let Ok(a_sel) = Selector::parse(&self.detail_author_selector) {
                author = doc.select(&a_sel).next()
                    .map(|a| a.text().collect::<Vec<_>>().join("").trim().to_string())
                    .filter(|s| !s.is_empty());
            }
        }

        // Fallback for Status if not found
        if status.is_none() {
            if let Ok(s_sel) = Selector::parse(&self.detail_status_selector) {
                status = doc.select(&s_sel).next()
                    .map(|s| s.text().collect::<Vec<_>>().join("").trim().to_string())
                    .filter(|s| !s.is_empty());
            }
        }

        // Fallback for Genres if not found
        if genres.is_empty() {
            if let Ok(g_sel) = Selector::parse(&self.detail_genre_selector) {
                for el in doc.select(&g_sel) {
                    let g = el.text().collect::<Vec<_>>().join("").trim().to_string();
                    if !g.is_empty() && !genres.contains(&g) {
                        genres.push(g);
                    }
                }
            }
        }

        // Description
        let description = Selector::parse(&self.detail_desc_selector).ok().and_then(|d_sel| {
            doc.select(&d_sel).next().map(|el| {
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
            }).filter(|s| !s.is_empty())
        });

        // Chapters fetching
        let mut chapters = Vec::new();
        let mut html_content = String::new();

        if self.use_new_chapter_endpoint {
            let ajax_url = format!("{}/ajax/chapters/", clean_url);
            let mut hdrs = HashMap::new();
            hdrs.insert("Referer".to_string(), clean_url.to_string());

            if let Ok(html) = host::post(&ajax_url, "", Some(hdrs.clone())) {
                html_content = html;
                let first_doc = Html::parse_fragment(&html_content);

                // Check for pagination: .pagination a[data-page]
                if let Ok(page_sel) = Selector::parse(".pagination a[data-page]") {
                    let mut max_page = 1;
                    let mut query_template = String::new();
                    for el in first_doc.select(&page_sel) {
                        if let Some(p_str) = el.value().attr("data-page") {
                            if let Ok(p) = p_str.parse::<i32>() {
                                if p > max_page { max_page = p; }
                            }
                        }
                        if query_template.is_empty() {
                            if let Some(href) = el.value().attr("href") {
                                if let Some(q_idx) = href.find('?') {
                                    let query = &href[q_idx..];
                                    let digits_len = query.chars().rev().take_while(|c| c.is_ascii_digit()).count();
                                    query_template = query[..query.len() - digits_len].to_string();
                                }
                            }
                        }
                    }

                    if max_page > 1 && !query_template.is_empty() {
                        for page in 2..=max_page {
                            let page_url = format!("{}/ajax/chapters/{}{}", clean_url, query_template, page);
                            if let Ok(page_html) = host::post(&page_url, "", Some(hdrs.clone())) {
                                if !page_html.is_empty() && page_html != "0" {
                                    html_content.push_str(&page_html);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Extract novel ID / post ID for wp-admin/admin-ajax.php
            let mut novel_id = String::new();
            if let Ok(id_sel) = Selector::parse(".rating-post-id, #manga-chapters-holder, [data-id]") {
                if let Some(el) = doc.select(&id_sel).next() {
                    novel_id = el.value().attr("value")
                        .or_else(|| el.value().attr("data-id"))
                        .unwrap_or("").to_string();
                }
            }

            if novel_id.is_empty() {
                let html = doc.html();
                for pat in &["rating-post-id\" value=\"", "data-id=\"", "post_id=\"", "post-id=\""] {
                    if let Some(idx) = html.find(pat) {
                        let rest = &html[idx + pat.len()..];
                        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                        if !digits.is_empty() {
                            novel_id = digits;
                            break;
                        }
                    }
                }
            }

            if !novel_id.is_empty() {
                let ajax_url = format!("{}/{}", self.base_url.trim_end_matches('/'), self.chapter_ajax_path.trim_start_matches('/'));
                let body = format!("action={}&manga={}", self.chapter_ajax_action, novel_id);
                let mut hdrs = HashMap::new();
                hdrs.insert("Content-Type".to_string(), "application/x-www-form-urlencoded; charset=UTF-8".to_string());

                if let Ok(html) = host::post(&ajax_url, &body, Some(hdrs)) {
                    if html != "0" && !html.is_empty() {
                        html_content = html;
                    }
                }
            }
        }

        let chapter_doc = if !html_content.is_empty() && html_content != "0" {
            Html::parse_fragment(&html_content)
        } else {
            doc
        };

        if let Ok(item_sel) = Selector::parse(&self.chapter_item_selector) {
            if let Ok(link_sel) = Selector::parse(&self.chapter_link_selector) {
                let date_sel = Selector::parse(&self.chapter_date_selector).ok();
                for item in chapter_doc.select(&item_sel) {
                    let Some(link) = item.select(&link_sel).next() else { continue; };
                    let href = link.value().attr("href").unwrap_or("");
                    if href.is_empty() || href == "#" { continue; }

                    let mut ch_title = link.text().collect::<Vec<_>>().join("").trim().to_string();

                    // Check if locked / premium block
                    let class_attr = item.value().attr("class").unwrap_or("");
                    if class_attr.contains("premium-block") || class_attr.contains("locked") {
                        ch_title = format!("🔒 {}", ch_title);
                    }

                    let release_date = date_sel.as_ref().and_then(|d_s| {
                        item.select(d_s).next().map(|d| d.text().collect::<Vec<_>>().join("").trim().to_string()).filter(|s| !s.is_empty())
                    });

                    chapters.push(ChapterDto {
                        url: self.abs_url(href),
                        title: ch_title,
                        index: 0,
                        release_date,
                        scanlation: None,
                    });
                }
            }
        }

        // Chapters in Madara are listed newest to oldest. Reverse to make earliest index 1.
        chapters.reverse();
        for (i, chap) in chapters.iter_mut().enumerate() {
            chap.index = (i + 1) as i32;
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
        let orderby = match listing_id {
            "latest" => "latest",
            "new" => "new-manga",
            _ => "views",
        };
        let url = format!(
            "{}/page/{}/?s=&post_type=wp-manga&m_orderby={}",
            self.base_url.trim_end_matches('/'), page, orderby
        );
        let doc = host::document(&url, None)?;
        Ok(self.parse_novel_list(&doc))
    }
}
