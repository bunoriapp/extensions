use bunori_sdk::*;
use std::collections::HashMap;

#[derive(Default)]
pub struct AsiaNovelSource;

impl AsiaNovelSource {
    fn default_headers() -> HashMap<String, String> {
        let mut hdrs = HashMap::new();
        hdrs.insert(
            "User-Agent".to_string(),
            "Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 Mobile/15E148 Safari/04.1".to_string()
        );
        hdrs
    }

    fn parse_cards(doc: &Html, base_url: &str) -> Result<Vec<SearchResultDto>, String> {
        let card_sel = Selector::parse("ul#search-result-list li.card, ul#list-of-stories li.card, li.card._story, li.card")
            .map_err(|e| e.to_string())?;
        let link_sel = Selector::parse("h3.card__title a, a[href*='/story/']").map_err(|e| e.to_string())?;
        let author_sel = Selector::parse(".card__by-author a, .card__footer-author a, a.author").map_err(|e| e.to_string())?;
        let img_sel = Selector::parse(".card__image img, img.wp-post-image, img").map_err(|e| e.to_string())?;

        let mut results = Vec::new();
        for card in doc.select(&card_sel) {
            let Some(story_link) = card.select(&link_sel).next() else { continue; };
            let Some(href) = story_link.value().attr("href") else { continue; };
            let url = if href.starts_with("http") { href.to_string() } else { format!("{}{}", base_url, href) };

            let title = story_link.text().collect::<Vec<_>>().join("").trim().to_string();
            if title.is_empty() {
                continue;
            }

            let cover_url = card.select(&img_sel).next()
                .and_then(|img| img.value().attr("src").or_else(|| img.value().attr("data-src")))
                .map(|s| if s.starts_with("http") { s.to_string() } else { format!("{}{}", base_url, s) });

            let author = card.select(&author_sel).next()
                .map(|a| a.text().collect::<Vec<_>>().join("").trim().to_string())
                .filter(|s| !s.is_empty());

            if !url.is_empty() && !results.iter().any(|r: &SearchResultDto| r.url == url) {
                results.push(SearchResultDto {
                    url,
                    title,
                    cover_url,
                    author,
                });
            }
        }

        Ok(results)
    }
}

impl Source for AsiaNovelSource {
    fn metadata(&self) -> SourceMetadata {
        serde_json::from_str(include_str!("../manifest.json"))
            .expect("Invalid manifest.json")
    }

    fn search(&self, query: &str, _page: i32) -> Result<Vec<SearchResultDto>, String> {
        let meta = self.metadata();
        let formatted = query.replace(' ', "+");
        let search_url = format!("{}/?s={}&post_type=fcn_story&sentence=0&orderby=modified&order=desc", meta.base_url, formatted);
        let doc = host::document(&search_url, Some(Self::default_headers()))?;
        Self::parse_cards(&doc, &meta.base_url)
    }

    fn get_novel_details(&self, novel_url: &str) -> Result<NovelDto, String> {
        let meta = self.metadata();
        let doc = host::document(novel_url, Some(Self::default_headers()))?;

        let og_title_sel = Selector::parse("meta[property='og:title']").map_err(|e| e.to_string())?;
        let author_meta_sel = Selector::parse("meta[property='article:author']").map_err(|e| e.to_string())?;
        let author_el_sel = Selector::parse("header.story__headline em.story__author a").map_err(|e| e.to_string())?;
        let og_img_sel = Selector::parse("meta[property='og:image']").map_err(|e| e.to_string())?;
        let og_desc_sel = Selector::parse("meta[property='og:description']").map_err(|e| e.to_string())?;

        let title = doc.select(&og_title_sel).next()
            .and_then(|m| m.value().attr("content"))
            .map(|s| s.split(" - Asianovel").next().unwrap_or(s).trim().to_string())
            .unwrap_or_default();

        let author = doc.select(&author_meta_sel).next()
            .and_then(|m| m.value().attr("content"))
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .or_else(|| doc.select(&author_el_sel).next().map(|a| a.text().collect::<Vec<_>>().join("").trim().to_string()));

        let cover_url = doc.select(&og_img_sel).next()
            .and_then(|m| m.value().attr("content"))
            .map(|s| s.to_string());

        let description = doc.select(&og_desc_sel).next()
            .and_then(|m| m.value().attr("content"))
            .map(|s| s.to_string());

        // Chapters from ld+json
        let mut chapters = Vec::new();
        let script_sel = Selector::parse("script[type='application/ld+json']").map_err(|e| e.to_string())?;
        for script in doc.select(&script_sel) {
            let script_content = script.text().collect::<Vec<_>>().join("");
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&script_content) {
                if let Some(graph) = json.get("@graph").and_then(|g| g.as_array()) {
                    for item in graph {
                        if item.get("@type").and_then(|t| t.as_str()) == Some("ItemList")
                            && item.get("name").and_then(|n| n.as_str()) == Some("Chapters")
                        {
                            if let Some(list) = item.get("itemListElement").and_then(|l| l.as_array()) {
                                for (j, chap_obj) in list.iter().enumerate() {
                                    let url = chap_obj.get("url").and_then(|u| u.as_str()).unwrap_or("");
                                    let pos = chap_obj.get("position").and_then(|p| p.as_i64()).unwrap_or((j + 1) as i64) as i32;
                                    let name = chap_obj.get("name").and_then(|n| n.as_str())
                                        .or_else(|| chap_obj.get("title").and_then(|t| t.as_str()))
                                        .unwrap_or("");
                                    let chap_title = if name.is_empty() { format!("Chapter {}", pos) } else { name.to_string() };

                                    if !url.is_empty() {
                                        chapters.push(ChapterDto {
                                             url: url.to_string(),
                                             title: chap_title,
                                             index: pos,
                                             release_date: None,
                                             scanlation: None,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if chapters.is_empty() {
            if let Ok(ch_sel) = Selector::parse("ol.chapter-group__list li.chapter-group__list-item a.chapter-group__list-item-link") {
                for (i, a) in doc.select(&ch_sel).enumerate() {
                    let href = a.value().attr("href").unwrap_or("");
                    let ch_url = if href.starts_with("http") { href.to_string() } else { format!("{}{}", meta.base_url, href) };
                    let title = a.text().collect::<Vec<_>>().join("").trim().to_string();
                    chapters.push(ChapterDto {
                        url: ch_url,
                        title,
                        index: (i + 1) as i32,
                        release_date: None,
                        scanlation: None,
                    });
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
            status: None,
            genres: Vec::new(),
            chapters,
            extra: HashMap::new(),
        })
    }

    fn get_chapter_content(&self, chapter_url: &str) -> Result<Option<String>, String> {
        let html = host::get(chapter_url, Some(Self::default_headers()))?;
        let doc = Html::parse_document(&html);
        let Ok(sel) = Selector::parse("#chapter-content") else {
            return Ok(None);
        };
        let Some(el) = doc.select(&sel).next() else {
            return Ok(None);
        };

        let mut content = el.inner_html();
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

    fn get_listings(&self) -> Vec<ListingDto> {
        vec![
            ListingDto { id: "updated".to_string(), name: "Latest Updates".to_string() },
            ListingDto { id: "published".to_string(), name: "New Stories".to_string() },
            ListingDto { id: "popular".to_string(), name: "Most Comments".to_string() },
            ListingDto { id: "words".to_string(), name: "Most Words".to_string() },
            ListingDto { id: "title".to_string(), name: "Alphabetical (A-Z)".to_string() },
        ]
    }

    fn get_listing_novels(&self, listing_id: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {
        let meta = self.metadata();
        let list_url = if listing_id.starts_with("genre_") {
            let slug = &listing_id[6..];
            if page <= 1 {
                format!("{}/genre/{}/", meta.base_url.trim_end_matches('/'), slug)
            } else {
                format!("{}/genre/{}/page/{}/", meta.base_url.trim_end_matches('/'), slug, page)
            }
        } else {
            let (orderby, order) = match listing_id {
                "published" | "new" => ("date", "desc"),
                "popular" | "comments" => ("comment_count", "desc"),
                "words" => ("words", "desc"),
                "title" | "az" => ("title", "asc"),
                _ => ("modified", "desc"), // "updated" / default
            };
            if page <= 1 {
                format!("{}/stories/?orderby={}&order={}", meta.base_url.trim_end_matches('/'), orderby, order)
            } else {
                format!("{}/stories/page/{}/?orderby={}&order={}", meta.base_url.trim_end_matches('/'), page, orderby, order)
            }
        };

        let doc = host::document(&list_url, Some(Self::default_headers()))?;
        Self::parse_cards(&doc, &meta.base_url)
    }
}

export_source!(AsiaNovelSource);
