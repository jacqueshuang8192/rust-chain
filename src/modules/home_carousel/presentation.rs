use super::domain::{HomeCarouselDocument, HomeCarouselSlide};
use crate::architecture::PresentationLayer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HomeCarouselQuery {
    pub locale: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PublicHomeCarouselResponse {
    pub slides: Vec<HomeCarouselSlide>,
}

impl PresentationLayer for PublicHomeCarouselResponse {}

#[derive(Debug, Serialize)]
pub struct AdminHomeCarouselResponse {
    pub revision: u64,
    pub slides: Vec<HomeCarouselSlide>,
}

impl PresentationLayer for AdminHomeCarouselResponse {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveHomeCarouselRequest {
    pub expected_revision: u64,
    pub slides: Vec<HomeCarouselSlide>,
    pub reason: String,
}

impl From<SaveHomeCarouselRequest> for HomeCarouselDocument {
    fn from(request: SaveHomeCarouselRequest) -> Self {
        Self {
            slides: request.slides,
        }
    }
}
