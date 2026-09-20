use bunori_sdk::*;
use madara_template::MadaraEngine;

#[derive(Default)]
pub struct WuxiaWorldSiteSource;

impl WuxiaWorldSiteSource {
    fn engine(&self) -> MadaraEngine {
        let meta = self.metadata();
        let mut engine = MadaraEngine::new(meta.base_url);
        engine.use_new_chapter_endpoint = true;
        engine
    }
}

impl Source for WuxiaWorldSiteSource {
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

export_source!(WuxiaWorldSiteSource);
