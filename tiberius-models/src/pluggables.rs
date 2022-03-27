
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Hashable {
    pub sha512_hash: Option<String>,
    pub orig_sha512_hash: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Intensities {
    pub ne: f32,
    pub nw: f32,
    pub se: f32,
    pub sw: f32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Representations {
    pub full: String,
    pub large: String,
    pub medium: String,
    pub mp4: String,
    pub small: String,
    pub tall: String,
    pub thumb: String,
    pub thumb_small: String,
    pub thumb_tiny: String,
    pub webm: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ImageUrls {
    pub representations: Representations,
    pub view_url: String,
    pub source_url: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ImageFileMetadata {
    pub thumbnails_generated: bool,
    pub processed: bool,
    pub duration: f32,
    pub mime_type: String,
    pub height: u64,
    pub size: u64,
    pub format: String,
    pub aspect_ratio: f32,
    pub intensities: Intensities,
    pub representations: Representations,
    pub animated: bool,
    pub width: u64,
    pub spoilered: bool,
    pub hidden_from_users: bool,
    pub duplicate_of: Option<u64>,
    pub deletion_reason: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ImageInteractionMetadata {
    pub upvotes: i64,
    pub faves: u64,
    pub downvotes: u64,
    pub comment_count: u64,
    pub tag_count: u64,
    pub tag_ids: Vec<u64>,
    pub score: i64,
    pub tags: Vec<String>,
    pub wilson_score: f32,

}