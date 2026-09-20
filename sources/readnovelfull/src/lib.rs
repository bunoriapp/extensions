use bunori_sdk::*;
use readnovelfull_template::ReadNovelFullEngine;

#[derive(Default)]
pub struct ReadNovelFullSource;

impl ReadNovelFullSource {
    fn engine(&self) -> ReadNovelFullEngine {
        let meta = self.metadata();
        let mut engine = ReadNovelFullEngine::new(meta.base_url);
        engine.search_path = "novel-list/search".into();
        engine.chapter_endpoint = "ajax/chapter-archive".into();
        engine.listings = vec![
            ListingDto { id: "novel-list/latest-release-novel".into(), name: "Latest Release".into() },
            ListingDto { id: "novel-list/hot-novel".into(), name: "Hot Novels".into() },
            ListingDto { id: "novel-list/completed-novel".into(), name: "Completed Novels".into() },
        ];
        engine
    }
}

impl Source for ReadNovelFullSource {
    fn metadata(&self) -> SourceMetadata {
        serde_json::from_str(include_str!("../manifest.json")).expect("Invalid manifest.json")
    }

    fn search(&self, query: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {
        self.engine().search(query, page)
    }

    fn get_novel_details(&self, novel_url: &str) -> Result<NovelDto, String> {
        self.engine().get_novel_details(novel_url)
    }

    fn get_chapter_content(&self, chapter_url: &str) -> Result<Option<String>, String> {
        self.engine().get_chapter_content(chapter_url)
    }

    fn get_listings(&self) -> Vec<ListingDto> {
        self.engine().get_listings()
    }

    fn get_listing_novels(&self, listing_id: &str, page: i32) -> Result<Vec<SearchResultDto>, String> {
        self.engine().get_listing_novels(listing_id, page)
    }
}

export_source!(ReadNovelFullSource);
