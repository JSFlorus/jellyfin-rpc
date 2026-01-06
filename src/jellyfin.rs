use std::collections::HashMap;
use serde::Deserialize;

 

#[derive(Debug, Deserialize)]
pub struct Session {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "UserName")]
    pub username: String,
    #[serde(rename = "NowPlayingItem")]
    pub now_playing_item: Option<NowPlayingItem>,
    #[serde(rename = "PlayState")]
    pub play_state: Option<PlayState>,
}

#[derive(Debug, Deserialize)]
pub struct PlayState {
    #[serde(rename = "PositionTicks")]
    pub position_ticks: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
 pub struct NowPlayingItem {
    #[serde(rename = "Id")]
    pub id: String,

    #[serde(rename = "Name")]
    pub name: String,

    #[serde(rename = "Artists")]
    pub artists: Option<Vec<String>>,

    #[serde(rename = "RunTimeTicks")]
    pub runtime_ticks: Option<i64>,

    #[serde(rename = "AlbumId")]
    pub album_id: Option<String>,

    #[serde(rename = "ParentId")]
    pub parent_id: Option<String>,

    #[serde(rename = "Type")]
    pub item_type: Option<String>,

    #[serde(rename = "SeriesId")]
    pub series_id: Option<String>,

    #[serde(rename = "SeasonId")]
    pub season_id: Option<String>,

    #[serde(rename = "SeriesName")]
    pub series_name: Option<String>,

    #[serde(rename = "ParentIndexNumber")]
    pub season_number: Option<i32>,

    #[serde(rename = "IndexNumber")]
    pub episode_number: Option<i32>,

    #[serde(rename = "ProductionYear")]
    pub production_year: Option<i32>,

    // New: lets us detect if the track has its own Primary image.
    // In your JSON this was like: "ImageTags": {"Primary":"..."} or {}
    #[serde(rename = "ImageTags")]
    pub image_tags: Option<HashMap<String, String>>,
}


pub fn pick_session_for_user(
    state: &mut crate::State,
    sessions: Vec<Session>,
    user: &str,
) -> Option<(String, Option<NowPlayingItem>, i64, bool)> {
    let mut best: Option<(String, Option<NowPlayingItem>, i64, bool)> = None;

    for s in sessions {
        if s.username.to_lowercase() != user.to_lowercase() {
            continue;
        }

        let sid = s.id.clone();
        let pos = s.play_state.and_then(|p| p.position_ticks).unwrap_or(0);
        let has_real_item = s.now_playing_item.is_some();
        let item_opt = s.now_playing_item.clone()
            .or_else(|| state.last_item_by_session.get(&sid).cloned());

        if let Some(ref real_it) = s.now_playing_item {
            state.last_item_by_session.insert(sid.clone(), real_it.clone());
            state.last_pos_by_session.insert(sid.clone(), pos);
        }

        match &mut best {
            None => best = Some((sid, item_opt, pos, has_real_item)),
            Some((_, _, best_pos, best_has_real)) => {
                if has_real_item && !*best_has_real {
                    best = Some((sid, item_opt, pos, true));
                } else if has_real_item == *best_has_real && pos > *best_pos {
                    best = Some((sid, item_opt, pos, has_real_item));
                }
            }
        }
    }

    best
}
