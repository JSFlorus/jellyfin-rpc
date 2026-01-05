use crate::jellyfin::NowPlayingItem;
use chrono::Utc;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient}; 
use serde_json::json;
use std::cmp::min;
use uuid::Uuid;
use crate::covers::get_cover_url;


pub fn set_activity(
    client: &mut DiscordIpcClient,
    item: &NowPlayingItem,
    elapsed: i64,
    runtime: i64,
) {
    let now = Utc::now().timestamp();

    let (details, state_str, activity_type) = match item.item_type.as_deref() {
        Some("Audio") => (
            item.name.clone(),
            item.artists.clone().unwrap_or_default().join(", "),
            2,
        ),
        Some("Movie") => {
            let s = item.production_year
                .map(|y| y.to_string())
                .unwrap_or_else(|| "Movie".to_string());
            (item.name.clone(), s, 3)
        }
        Some("Episode") => {
            let details = item.name.clone();
            let s = if let (Some(season), Some(ep), Some(series)) =
                (item.season_number, item.episode_number, &item.series_name)
            {
                format!("S{:02}E{:02} – {}", season, ep, series)
            } else {
                item.series_name.clone().unwrap_or_else(|| "TV Show".to_string())
            };
            (details, s, 3)
        }
        _ => (item.name.clone(), String::new(), 3),
    };

    let cover_url = get_cover_url(item).unwrap_or_else(|| "default".to_string());
    let elapsed_clamped = if runtime > 0 { min(elapsed, runtime) } else { elapsed };

    let payload = json!({
        "cmd": "SET_ACTIVITY",
        "args": {
            "activity": {
                "details": details,
                "state": state_str,
                "assets": { "large_image": cover_url },
                "timestamps": {
                    "start": now - elapsed_clamped,
                    "end": if runtime > 0 { now - elapsed_clamped + runtime } else { now - elapsed_clamped + 1 }
                },
                "type": activity_type
            },
            "pid": std::process::id()
        },
        "nonce": Uuid::new_v4().to_string()
    });

    let _ = client.send(payload, 1);
    println!("▶ Activity: {} [{}] (elapsed {}s)", details, state_str, elapsed_clamped);
}

pub fn clear_discord(client: &mut DiscordIpcClient) {
    let payload = json!({
        "cmd": "SET_ACTIVITY",
        "args": { "activity": null, "pid": std::process::id() },
        "nonce": Uuid::new_v4().to_string()
    });
    let _ = client.send(payload, 1);
}
