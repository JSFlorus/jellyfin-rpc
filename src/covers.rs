use crate::{URL, API_KEY};
use crate::NowPlayingItem;

fn has_primary(item: &NowPlayingItem) -> bool {
    item.image_tags
        .as_ref()
        .and_then(|t| t.get("Primary"))
        .is_some()
}

pub fn get_cover_url(item: &NowPlayingItem) -> Option<String> {
    let base = format!("{}/Items", URL.as_str());
    let qp = format!("maxWidth=512&quality=90&api_key={}", API_KEY.as_str());

    match item.item_type.as_deref()? {
        "Audio" => {
            // track primary if present, else album
            if has_primary(item) {
                return Some(format!("{}/{}/Images/Primary?{}", base, item.id, qp));
            }
            if let Some(album_id) = &item.album_id {
                return Some(format!("{}/{}/Images/Primary?{}", base, album_id, qp));
            }
            if let Some(parent_id) = &item.parent_id {
                return Some(format!("{}/{}/Images/Primary?{}", base, parent_id, qp));
            }
            None
        }

        "Episode" => {
            // 1) Prefer Season image (most consistent for anime/TV)
            if let Some(season_id) = &item.season_id {
                return Some(format!("{}/{}/Images/Primary?{}", base, season_id, qp));
            }

            // 2) Fall back to Series image
            if let Some(series_id) = &item.series_id {
                return Some(format!("{}/{}/Images/Primary?{}", base, series_id, qp));
            }

            // 3) Last resort: Episode image (only if it actually exists)
            if has_primary(item) {
                return Some(format!("{}/{}/Images/Primary?{}", base, item.id, qp));
            }

            None
        }

        "Movie" => {
            // Movie primary if present, else try parent (rare but happens with some libraries)
            if has_primary(item) {
                return Some(format!("{}/{}/Images/Primary?{}", base, item.id, qp));
            }
            if let Some(parent_id) = &item.parent_id {
                return Some(format!("{}/{}/Images/Primary?{}", base, parent_id, qp));
            }
            // last resort: still try the movie id (some servers don't populate tags but do serve art)
            Some(format!("{}/{}/Images/Primary?{}", base, item.id, qp))
        }

        _ => Some(format!("{}/{}/Images/Primary?{}", base, item.id, qp)),
    }
}
