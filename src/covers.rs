use crate::{URL, API_KEY};
use crate::NowPlayingItem;

pub fn get_cover_url(item: &NowPlayingItem) -> Option<String> {
    let base = format!("{}/Items", URL.as_str());
    let qp = format!("maxWidth=512&quality=90&api_key={}", API_KEY.as_str());

    match item.item_type.as_deref()? {
        "Audio" => {
            Some(format!("{}/{}/Images/Primary?{}", base, item.id, qp))
        }
        "Episode" => {
            if let Some(season_id) = &item.season_id {
                return Some(format!("{}/{}/Images/Primary?{}", base, season_id, qp));
            }
            if let Some(series_id) = &item.series_id {
                return Some(format!("{}/{}/Images/Primary?{}", base, series_id, qp));
            }
            Some(format!("{}/{}/Images/Primary?{}", base, item.id, qp))
        }
        "Movie" => Some(format!("{}/{}/Images/Primary?{}", base, item.id, qp)),
        _ => Some(format!("{}/{}/Images/Primary?{}", base, item.id, qp)),
    }
}
