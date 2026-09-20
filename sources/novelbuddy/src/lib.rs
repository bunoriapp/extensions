use bunori_sdk::*;
use serde::Deserialize;
use std::collections::HashMap;

const API_BASE: &str = "https://api.novelbuddy.me";

#[derive(Default)]
pub struct NovelBuddySource;

#[derive(Deserialize)]
struct SearchResponse {
    data: Option<SearchData>,
}

#[derive(Deserialize)]
struct SearchData {
    items: Option<Vec<SearchItem>>,
}

#[derive(Deserialize)]
struct SearchItem {
    name: Option<String>,
    url: Option<String>,
    cover: Option<String>,
}

#[derive(Deserialize)]
struct NextData {
    props: Option<NextProps>,
}

#[derive(Deserialize)]
struct NextProps {
    #[serde(rename = "pageProps")]
    page_props: Option<PageProps>,
}

#[derive(Deserialize)]
struct PageProps {
    #[serde(rename = "initialManga")]
    initial_manga: Option<InitialManga>,
    #[serde(rename = "initialChapter")]
    initial_chapter: Option<InitialChapter>,
}

#[derive(Deserialize)]
struct InitialManga {
    id: String,
    name: Option<String>,
    cover: Option<String>,
    status: Option<String>,
    summary: Option<String>,
    authors: Option<Vec<NamedEntity>>,
    genres: Option<Vec<NamedEntity>>,
}

#[derive(Deserialize)]
struct NamedEntity {
    name: Option<String>,
}

#[derive(Deserialize)]
struct InitialChapter {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ChaptersApiResponse {
    data: Option<ChaptersApiData>,
}

#[derive(Deserialize)]
struct ChaptersApiData {
    chapters: Option<Vec<ChapterApiItem>>,
}

#[derive(Deserialize)]
struct ChapterApiItem {
    id: String,
    name: Option<String>,
    url: Option<String>,
    number: Option<f64>,
    updated_at: Option<String>,
}

#[derive(Deserialize)]
struct ChapterContentApiResponse {
    data: Option<ChapterContentApiData>,
}

#[derive(Deserialize)]
struct ChapterContentApiData {
    chapter: Option<ChapterDetailItem>,
}

#[derive(Deserialize)]
struct ChapterDetailItem {
    content: Option<String>,
}

impl NovelBuddySource {
    fn clean_content(mut raw: String) -> String {
        if let Some(pos) = raw.find("Find authorized novels in Webnovel") {
            raw.truncate(pos);
        }
        raw.trim().to_string()
    }
}

impl Source for NovelBuddySource {
    fn metadata(&self) -> SourceMetadata {
        serde_json::from_str(include_str!("../manifest.json"))
            .expect("Invalid manifest.json")
    }

    fn search(&self, query: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {
        let meta = self.metadata();
        let encoded_query = query.replace(' ', "%20");
        let search_url = format!("{}/titles/search?limit=24&page={}&q={}", API_BASE, page, encoded_query);

        let body = host::get(&search_url, None)?;
        let parsed: SearchResponse = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse search JSON: {}", e))?;

        let mut results = Vec::new();
        if let Some(data) = parsed.data {
            if let Some(items) = data.items {
                for item in items {
                    let title = item.name.unwrap_or_default().trim().to_string();
                    let relative_url = item.url.unwrap_or_default();
                    if title.is_empty() || relative_url.is_empty() {
                        continue;
                    }
                    let url = if relative_url.starts_with("http") {
                        relative_url
                    } else {
                        format!("{}{}", meta.base_url, relative_url)
                    };

                    results.push(SearchResultDto {
                        url,
                        title,
                        cover_url: item.cover,
                        author: None,
                    });
                }
            }
        }

        Ok(results)
    }

    fn get_novel_details(&self, novel_url: &str) -> Result<NovelDto, String> {
        let meta = self.metadata();
        let html = host::get(novel_url, None)?;

        // 1. Extract __NEXT_DATA__
        let script_re = Regex::new(r#"<script id="__NEXT_DATA__"[^>]*>(.*?)</script>"#)
            .map_err(|e| e.to_string())?;

        let Some(caps) = script_re.captures(&html) else {
            return Err(format!("Could not find __NEXT_DATA__ script in {}", novel_url));
        };

        let next_data_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let next_data: NextData = serde_json::from_str(next_data_str)
            .map_err(|e| format!("Failed to parse __NEXT_DATA__: {}", e))?;

        let manga = next_data.props
            .and_then(|p| p.page_props)
            .and_then(|pp| pp.initial_manga)
            .ok_or_else(|| "Could not find initialManga in pageProps".to_string())?;

        let title = manga.name.unwrap_or_default().trim().to_string();
        let cover_url = manga.cover;
        let author = manga.authors.map(|authors| {
            authors.into_iter().filter_map(|a| a.name).collect::<Vec<_>>().join(", ")
        }).filter(|s| !s.is_empty());

        let genres = manga.genres.map(|genres| {
            genres.into_iter().filter_map(|g| g.name).collect::<Vec<_>>()
        }).unwrap_or_default();

        let status = manga.status.map(|s| match s.to_lowercase().as_str() {
            "ongoing" => "Ongoing".to_string(),
            "completed" => "Completed".to_string(),
            "hiatus" => "On Hiatus".to_string(),
            "cancelled" | "dropped" => "Cancelled".to_string(),
            _ => s,
        });

        let description = manga.summary.map(|s| {
            s.replace("<p>", "").replace("</p>", "\n").replace("<br />", "\n").replace("<br>", "\n").trim().to_string()
        });

        // 2. Fetch all chapters from API
        let novel_id = manga.id;
        let chapters_url = format!("{}/titles/{}/chapters", API_BASE, novel_id);
        let chapters_body = host::get(&chapters_url, None)?;
        let chapters_parsed: ChaptersApiResponse = serde_json::from_str(&chapters_body)
            .map_err(|e| format!("Failed to parse chapters JSON: {}", e))?;

        let mut chapters = Vec::new();
        if let Some(data) = chapters_parsed.data {
            if let Some(mut items) = data.chapters {
                // Chapters API returns newest first -> reverse to chronological order
                items.reverse();

                for (idx, ch) in items.into_iter().enumerate() {
                    let ch_title = ch.name.unwrap_or_else(|| {
                        if let Some(num) = ch.number {
                            format!("Chapter {}", num)
                        } else {
                            "Chapter".to_string()
                        }
                    });

                    let rel_url = ch.url.unwrap_or_default();
                    let full_url = if rel_url.starts_with("http") {
                        format!("{}?id={}&chapterId={}", rel_url, novel_id, ch.id)
                    } else {
                        format!("{}{}{}?id={}&chapterId={}", meta.base_url, if rel_url.starts_with('/') { "" } else { "/" }, rel_url, novel_id, ch.id)
                    };

                    chapters.push(ChapterDto {
                        url: full_url,
                        title: ch_title.trim().to_string(),
                        index: idx as i32,
                        release_date: ch.updated_at,
                        scanlation: None,
                    });
                }
            }
        }

        Ok(NovelDto {
            url: novel_url.to_string(),
            title,
            cover_url,
            author,
            description,
            status,
            genres,
            chapters,
            extra: HashMap::new(),
        })
    }

    fn get_chapter_content(&self, url: &str) -> Result<Option<String>, String> {
        let mut novel_id = None;
        let mut chapter_id = None;
        if let Some(query_str) = url.split('?').nth(1) {
            for param in query_str.split('&') {
                if let Some((k, v)) = param.split_once('=') {
                    if k == "id" {
                        novel_id = Some(v.to_string());
                    } else if k == "chapterId" {
                        chapter_id = Some(v.to_string());
                    }
                }
            }
        }

        // 1. Try API first if IDs are available
        if let (Some(nid), Some(cid)) = (novel_id, chapter_id) {
            let api_url = format!("{}/titles/{}/chapters/{}", API_BASE, nid, cid);
            if let Ok(body) = host::get(&api_url, None) {
                if let Ok(resp) = serde_json::from_str::<ChapterContentApiResponse>(&body) {
                    if let Some(content) = resp.data.and_then(|d| d.chapter).and_then(|c| c.content) {
                        return Ok(Some(Self::clean_content(content)));
                    }
                }
            }
        }

        // 2. Fallback to HTML page and __NEXT_DATA__
        let clean_url = url.split('?').next().unwrap_or(url);
        let html = host::get(clean_url, None)?;
        let script_re = Regex::new(r#"<script id="__NEXT_DATA__"[^>]*>(.*?)</script>"#)
            .map_err(|e| e.to_string())?;

        if let Some(caps) = script_re.captures(&html) {
            let next_data_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            if let Ok(next_data) = serde_json::from_str::<NextData>(next_data_str) {
                if let Some(content) = next_data.props
                    .and_then(|p| p.page_props)
                    .and_then(|pp| pp.initial_chapter)
                    .and_then(|ic| ic.content)
                {
                    return Ok(Some(Self::clean_content(content)));
                }
            }
        }

        // 3. Fallback to DOM selector if rendered
        let doc = Html::parse_document(&html);
        if let Ok(sel) = Selector::parse(".chapter-content, #chapter-content, .content-inner") {
            if let Some(el) = doc.select(&sel).next() {
                let text = el.inner_html();
                return Ok(Some(Self::clean_content(text)));
            }
        }

        Err(format!("Could not extract chapter content for {}", url))
    }

    fn get_listings(&self) -> Vec<ListingDto> {
        vec![
            ListingDto {
                id: "popular".to_string(),
                name: "Most Popular".to_string(),
            },
            ListingDto {
                id: "latest".to_string(),
                name: "Latest Updates".to_string(),
            },
            ListingDto {
                id: "views".to_string(),
                name: "Most Viewed".to_string(),
            },
            ListingDto {
                id: "rating".to_string(),
                name: "Highest Rating".to_string(),
            },
        ]
    }

    fn get_listing_novels(&self, listing_id: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {
        let meta = self.metadata();
        let sort = match listing_id {
            "popular" => "popular",
            "latest" => "latest",
            "views" => "views",
            "rating" => "rating",
            _ => "views",
        };

        let url = format!("{}/titles/search?sort={}&page={}&limit=24", API_BASE, sort, page);
        let body = host::get(&url, None)?;
        let parsed: SearchResponse = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse listing JSON: {}", e))?;

        let mut results = Vec::new();
        if let Some(data) = parsed.data {
            if let Some(items) = data.items {
                for item in items {
                    let title = item.name.unwrap_or_default().trim().to_string();
                    let relative_url = item.url.unwrap_or_default();
                    if title.is_empty() || relative_url.is_empty() {
                        continue;
                    }
                    let novel_url = if relative_url.starts_with("http") {
                        relative_url
                    } else {
                        format!("{}{}", meta.base_url, relative_url)
                    };

                    results.push(SearchResultDto {
                        url: novel_url,
                        title,
                        cover_url: item.cover,
                        author: None,
                    });
                }
            }
        }

        Ok(results)
    }
}

export_source!(NovelBuddySource);
