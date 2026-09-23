use bunori_sdk::*;
use std::collections::HashMap;

pub struct NovelFireEngine {
    pub base_url: String,
    pub novel_path_prefix: String,

    pub search_endpoint: String,
    pub search_query_param: String,
    pub search_type_param: String,
    pub search_json_title_key: String,
    pub search_json_slug_key: String,
    pub search_json_image_key: String,

    pub chapter_ajax_endpoint: String,
    pub chapter_list_path: String,
    pub listing_path: String,
    pub listing_status_path: String,

    pub list_item_selector: String,
    pub list_link_selector: String,
    pub list_cover_selector: String,

    pub detail_title_selector: String,
    pub detail_author_selector: String,
    pub detail_cover_selector: String,
    pub detail_desc_selector: String,
    pub detail_genre_selector: String,
    pub detail_status_selector: String,
    pub detail_post_id_selector: String,
    pub detail_post_id_attr: String,

    pub chapter_list_selector: String,

    pub chapter_content_selector: String,

    pub listings: Vec<ListingDto>,
}

impl NovelFireEngine {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            novel_path_prefix: "novel".into(),

            search_endpoint: "ajax/searchLive".into(),
            search_query_param: "keyword".into(),
            search_type_param: "type=title".into(),
            search_json_title_key: "title".into(),
            search_json_slug_key: "slug".into(),
            search_json_image_key: "image".into(),

            chapter_ajax_endpoint: "ajax/listChapterDataAjax".into(),
            chapter_list_path: "chapters".into(),
            listing_path: "genre-all/sort".into(),
            listing_status_path: "status-all/all-novel".into(),

            list_item_selector: ".novel-item, .item".into(),
            list_link_selector: ".item-body h4 a, h4.novel-title a, h4 a, .item-title a, a".into(),
            list_cover_selector: "figure.cover img, figure.novel-cover img, img.cover, img".into(),

            detail_title_selector: "h1.novel-title".into(),
            detail_author_selector: ".author a[itemprop='author'], .author span[itemprop='author'], .author a".into(),
            detail_cover_selector: ".fixed-img figure.cover img, .glass-background img, .cover img".into(),
            detail_desc_selector: ".summary .content, [itemprop='description']".into(),
            detail_genre_selector: ".categories ul li a, .categories li a".into(),
            detail_status_selector: ".header-stats strong.ongoing, .header-stats strong.completed, .header-stats span strong".into(),
            detail_post_id_selector: "#novel-report, [report-post_id], [data-id], input#post_id".into(),
            detail_post_id_attr: "report-post_id".into(),

            chapter_list_selector: "ul.chapter-list li a, .chapter-list a".into(),

            chapter_content_selector: "#chapter-content, #content, .content, .chapter-body, .chapter-text".into(),

            listings: vec![
                ListingDto { id: "popular".into(), name: "Popular".into() },
                ListingDto { id: "new".into(), name: "New Novels".into() },
                ListingDto { id: "top-rated".into(), name: "Top Rated".into() },
                ListingDto { id: "view".into(), name: "Most Viewed".into() },
                ListingDto { id: "completed".into(), name: "Completed".into() },
            ],
        }
    }

    fn abs_url(&self, href: &str) -> String {
        if href.starts_with("http") {
            href.to_string()
        } else {
            format!("{}/{}", self.base_url.trim_end_matches('/'), href.trim_start_matches('/'))
        }
    }

    pub fn search(&self, query: &str) -> Result<Vec<SearchResultDto>, String> {
        let formatted = query.replace(' ', "%20");
        let search_url = if self.search_type_param.is_empty() {
            format!(
                "{}/{}?{}={}",
                self.base_url.trim_end_matches('/'), self.search_endpoint.trim_start_matches('/'),
                self.search_query_param, formatted
            )
        } else {
            format!(
                "{}/{}?{}={}&{}",
                self.base_url.trim_end_matches('/'), self.search_endpoint.trim_start_matches('/'),
                self.search_query_param, formatted, self.search_type_param
            )
        };
        let resp = host::get(&search_url, None)?;

        let Ok(json) = serde_json::from_str::<serde_json::Value>(&resp) else {
            return Ok(Vec::new());
        };

        let Some(data_arr) = json.get("data").and_then(|d| d.as_array()) else {
            return Ok(Vec::new());
        };

        let mut results = Vec::new();
        for item in data_arr {
            let title = item.get(&self.search_json_title_key).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let slug = item.get(&self.search_json_slug_key).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let image = item.get(&self.search_json_image_key).and_then(|v| v.as_str()).unwrap_or("");

            if !title.is_empty() && !slug.is_empty() {
                let cover_url = if !image.is_empty() {
                    Some(self.abs_url(image))
                } else {
                    None
                };
                let novel_url = if slug.starts_with("http") {
                    slug
                } else if self.novel_path_prefix.is_empty() {
                    format!("{}/{}", self.base_url.trim_end_matches('/'), slug.trim_start_matches('/'))
                } else {
                    format!("{}/{}/{}", self.base_url.trim_end_matches('/'), self.novel_path_prefix.trim_matches('/'), slug.trim_start_matches('/'))
                };

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

    pub fn get_novel_details(&self, novel_url: &str) -> Result<NovelDto, String> {
        let clean_url = novel_url.trim_end_matches('/');
        let doc = host::document(clean_url, None)?;

        let title_sel = Selector::parse(&self.detail_title_selector).map_err(|e| e.to_string())?;
        let author_sel = Selector::parse(&self.detail_author_selector).map_err(|e| e.to_string())?;
        let cover_sel = Selector::parse(&self.detail_cover_selector).map_err(|e| e.to_string())?;
        let desc_sel = Selector::parse(&self.detail_desc_selector).map_err(|e| e.to_string())?;
        let genre_sel = Selector::parse(&self.detail_genre_selector).map_err(|e| e.to_string())?;
        let status_sel = Selector::parse(&self.detail_status_selector).map_err(|e| e.to_string())?;

        let title = doc.select(&title_sel).next()
            .map(|t| t.text().collect::<Vec<_>>().join("").trim().to_string())
            .unwrap_or_default();

        let author = doc.select(&author_sel).next()
            .map(|a| a.text().collect::<Vec<_>>().join("").trim().to_string())
            .filter(|s| !s.is_empty());

        let cover_url = doc.select(&cover_sel).next()
            .and_then(|img| img.value().attr("src").or_else(|| img.value().attr("data-src")))
            .map(|s| self.abs_url(s));

        let description = doc.select(&desc_sel).next()
            .map(|d| d.text().collect::<Vec<_>>().join("").trim().to_string())
            .filter(|s| !s.is_empty());

        let mut genres = Vec::new();
        for el in doc.select(&genre_sel) {
            let g = el.text().collect::<Vec<_>>().join("").trim().to_string();
            if !g.is_empty() && !genres.contains(&g) {
                genres.push(g);
            }
        }

        let status = doc.select(&status_sel).next()
            .map(|s| s.text().collect::<Vec<_>>().join("").trim().to_string())
            .filter(|s| !s.is_empty());

        // Chapters
        let mut post_id = String::new();
        if let Ok(id_sel) = Selector::parse(&self.detail_post_id_selector) {
            if let Some(el) = doc.select(&id_sel).next() {
                post_id = el.value().attr(&self.detail_post_id_attr)
                    .or_else(|| el.value().attr("data-id"))
                    .or_else(|| el.value().attr("value"))
                    .unwrap_or("").to_string();
            }
        }
        if post_id.is_empty() {
            let html = doc.html();
            for pat in &[&format!("{}=", self.detail_post_id_attr), "report-post_id=", "post_id=", "data-id="] {
                if let Some(idx) = html.find(pat) {
                    let rest = &html[idx + pat.len()..];
                    let rest = rest.trim_start_matches(|c| c == '"' || c == '\'');
                    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                    if !digits.is_empty() {
                        post_id = digits;
                        break;
                    }
                }
            }
        }

        let mut chapters = Vec::new();
        if !post_id.is_empty() {
            let ajax_url = format!(
                "{}/{}?draw=1&start=0&length=-1&post_id={}&order[0][column]=0&order[0][dir]=asc&order[0][name]=cmm_posts_detail.n_sort&columns[0][data]=n_sort&columns[0][name]=cmm_posts_detail.n_sort&columns[0][searchable]=true&columns[0][orderable]=true&columns[0][search][value]=&columns[0][search][regex]=false&columns[1][data]=bookmark_created_at&columns[1][name]=bookmark_chapters.created_at&columns[1][searchable]=false&columns[1][orderable]=true&columns[1][search][value]=&columns[1][search][regex]=false&search[value]=&search[regex]=false&only_bookmark=false",
                self.base_url.trim_end_matches('/'), self.chapter_ajax_endpoint.trim_start_matches('/'), post_id
            );
            let mut hdrs = HashMap::new();
            hdrs.insert("Referer".to_string(), clean_url.to_string());
            hdrs.insert("X-Requested-With".to_string(), "XMLHttpRequest".to_string());

            if let Ok(resp) = host::get(&ajax_url, Some(hdrs)) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&resp) {
                    if let Some(arr) = json.get("data").and_then(|d| d.as_array()) {
                        for (i, item) in arr.iter().enumerate() {
                            let raw_title = item.get("title").and_then(|v| v.as_str())
                                .or_else(|| item.get("slug").and_then(|v| v.as_str()))
                                .unwrap_or("");
                            let n_sort = item.get("n_sort").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;

                            let chap_url = if n_sort > 0 {
                                format!("{}/chapter-{}", clean_url, n_sort)
                            } else {
                                let slug = item.get("slug").and_then(|v| v.as_str()).unwrap_or("");
                                if !slug.is_empty() {
                                    format!("{}/{}", clean_url, slug)
                                } else {
                                    continue;
                                }
                            };

                            let idx = if n_sort > 0 { n_sort } else { (i + 1) as i32 };
                            let title = if raw_title.is_empty() { format!("Chapter {}", idx) } else { raw_title.to_string() };

                            chapters.push(ChapterDto {
                                url: chap_url,
                                title,
                                index: idx,
                                release_date: None,
                                scanlation: None,
                            });
                        }
                    }
                }
            }
        }

        if chapters.is_empty() {
            let list_url = format!("{}/{}", clean_url, self.chapter_list_path.trim_start_matches('/'));
            if let Ok(chap_doc) = host::document(&list_url, None) {
                if let Ok(a_sel) = Selector::parse(&self.chapter_list_selector) {
                    for (i, a) in chap_doc.select(&a_sel).enumerate() {
                        let href = a.value().attr("href").unwrap_or("");
                        let chap_url = self.abs_url(href);
                        let title = a.text().collect::<Vec<_>>().join("").trim().to_string();
                        chapters.push(ChapterDto {
                            url: chap_url,
                            title: if title.is_empty() { format!("Chapter {}", i + 1) } else { title },
                            index: (i + 1) as i32,
                            release_date: None,
                            scanlation: None,
                        });
                    }
                }
            }
        }

        chapters.sort_by_key(|c| c.index);

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
        let (sort, status) = if listing_id == "completed" {
            ("popular", "status-completed")
        } else {
            (listing_id, "status-all")
        };
        let list_url = format!(
            "{}/genre-all/sort-{}/{}/all-novel?page={}",
            self.base_url.trim_end_matches('/'), sort, status, page
        );
        let doc = host::document(&list_url, None)?;

        let item_sel = Selector::parse(&self.list_item_selector).map_err(|e| e.to_string())?;
        let link_sel = Selector::parse(&self.list_link_selector).map_err(|e| e.to_string())?;
        let cover_sel = Selector::parse(&self.list_cover_selector).map_err(|e| e.to_string())?;
        let title_sel = Selector::parse("h4.novel-title, h4, .item-title").map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for item in doc.select(&item_sel) {
            let mut title = String::new();
            let mut url = String::new();

            if let Some(h) = item.select(&title_sel).next() {
                let t = h.text().collect::<Vec<_>>().join("").trim().to_string();
                if !t.is_empty() {
                    title = t;
                }
            }

            for a in item.select(&link_sel) {
                if title.is_empty() {
                    let title_attr = a.value().attr("title").unwrap_or("").trim().to_string();
                    if !title_attr.is_empty() {
                        title = title_attr;
                    } else {
                        let text = a.text().collect::<Vec<_>>().join("").trim().to_string();
                        if !text.is_empty() {
                            title = text;
                        }
                    }
                }

                if let Some(h) = a.value().attr("href") {
                    if !h.trim().is_empty() && h != "#" {
                        url = self.abs_url(h);
                        break;
                    }
                }
            }

            let cover_url = item.select(&cover_sel).next()
                .and_then(|img| {
                    img.value().attr("data-src")
                        .or_else(|| img.value().attr("data-original"))
                        .or_else(|| img.value().attr("src"))
                })
                .filter(|s| !s.starts_with("data:"))
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

        Ok(results)
    }
}
