//#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod covers;
mod env_utils;
mod jellyfin;
mod discord;

use crate::env_utils::load_local_env;
use crate::jellyfin::{pick_session_for_user, Session, NowPlayingItem};
use crate::discord::{set_activity, clear_discord};

use discord_rich_presence::{DiscordIpc, DiscordIpcClient}; // trait + client
use once_cell::sync::Lazy;
use reqwest::blocking::Client;
use std::{env, thread};
use std::time::{Duration, Instant};
use std::collections::HashMap;

// ------------ Config from ENV ------------
static DISCORD_CLIENT_ID: Lazy<String> =
    Lazy::new(|| env::var("DISCORD_CLIENT_ID").expect("Missing DISCORD_CLIENT_ID"));
static URL: Lazy<String> =
    Lazy::new(|| env::var("JELLYFIN_URL").expect("Missing JELLYFIN_URL"));
static API_KEY: Lazy<String> =
    Lazy::new(|| env::var("JELLYFIN_API_KEY").expect("Missing JELLYFIN_API_KEY"));
static USER: Lazy<String> =
    Lazy::new(|| env::var("JELLYFIN_USER").expect("Missing JELLYFIN_USER"));

static JELLYFIN_POLL_INTERVAL_SECS: Lazy<u64> =
    Lazy::new(|| env::var("JELLYFIN_POLL_INTERVAL_SECS").unwrap_or_else(|_| "1".to_string()).parse().unwrap());
static DISCORD_UPDATE_INTERVAL_SECS: Lazy<u64> =
    Lazy::new(|| env::var("DISCORD_UPDATE_INTERVAL_SECS").unwrap_or_else(|_| "5".to_string()).parse().unwrap());
static NULL_GAP_REWIND_SECS: Lazy<i64> =
    Lazy::new(|| env::var("NULL_GAP_REWIND_SECS").unwrap_or_else(|_| "40".to_string()).parse().unwrap());
static NULL_GAP_MAX_SECS: Lazy<u64> =
    Lazy::new(|| env::var("NULL_GAP_MAX_SECS").unwrap_or_else(|_| "40".to_string()).parse().unwrap());

// ------------ State tracking ------------
pub struct State {
    pub last_pos_by_session: HashMap<String, i64>,
    pub last_item_by_session: HashMap<String, NowPlayingItem>,
    pub active_session_id: Option<String>,
    pub null_since_by_session: HashMap<String, Instant>,
    pub null_base_elapsed_by_session: HashMap<String, i64>,
    pub rewind_done_by_session: HashMap<String, bool>,
}

impl State {
    pub fn new() -> Self {
        Self {
            last_pos_by_session: HashMap::new(),
            last_item_by_session: HashMap::new(),
            active_session_id: None,
            null_since_by_session: HashMap::new(),
            null_base_elapsed_by_session: HashMap::new(),
            rewind_done_by_session: HashMap::new(),
        }
    }
}

// ------------ Main ------------
fn main() {
    load_local_env();

    eprintln!("Loaded Discord Client ID: '{}'", *DISCORD_CLIENT_ID);
    let client = Client::new();
    let mut discord =
        DiscordIpcClient::new(DISCORD_CLIENT_ID.as_str()).expect("Failed to create Discord client");
    discord.connect().expect("Failed to connect to Discord IPC");

    let mut state = State::new();
    let mut last_discord_update = Instant::now();

    loop {
        let resp = client
            .get(&format!("{}/Sessions", URL.as_str()))
            .header("X-Emby-Token", API_KEY.as_str())
            .send();

        if let Ok(r) = resp {
            if let Ok(sessions) = r.json::<Vec<Session>>() {
                if let Some((sid, item_opt, pos_ticks, had_real_item)) =
                    pick_session_for_user(&mut state, sessions, USER.as_str())
                {
                    if had_real_item {
                        state.null_since_by_session.remove(&sid);
                        state.null_base_elapsed_by_session.remove(&sid);
                        state.rewind_done_by_session.remove(&sid); // reset rewind

                        let runtime_secs = item_opt
                            .as_ref()
                            .and_then(|it| it.runtime_ticks)
                            .unwrap_or(0) / 10_000_000;
                        let elapsed_secs = pos_ticks / 10_000_000;

                        if last_discord_update.elapsed().as_secs() >= *DISCORD_UPDATE_INTERVAL_SECS {
                            if elapsed_secs == 0 {
                                clear_discord(&mut discord);
                                state.active_session_id = None;
                                println!("⏸ Paused or stopped: cleared activity");
                            } else if let Some(ref item) = item_opt {
                                set_activity(&mut discord, item, elapsed_secs, runtime_secs);
                                state.active_session_id = Some(sid.clone());
                            }
                            last_discord_update = Instant::now();
                        }
                    } else {
                        if let Some(last_item) = state.last_item_by_session.get(&sid).cloned() {
                            let last_ticks = *state.last_pos_by_session.get(&sid).unwrap_or(&0);
                            let last_elapsed = last_ticks / 10_000_000;
                            let rt_secs = last_item.runtime_ticks.unwrap_or(0) / 10_000_000;

                            let since = state.null_since_by_session.entry(sid.clone()).or_insert_with(Instant::now);
                            let gap_secs = since.elapsed().as_secs();

                            // === 3 CASES ===
                            if rt_secs < *NULL_GAP_REWIND_SECS as i64 {
                                // Case 1: Short track (<40s) → hold only
                                println!("🎵 Case 1: Short track (<40s). Holding at {}s", last_elapsed);
                                // no updates, just wait for grace expiry
                            } else if last_elapsed < *NULL_GAP_REWIND_SECS {
                                // Case 2: Long track but stopped before 40s → hold only
                                println!("🎵 Case 2: Early stop (<40s). Holding at {}s", last_elapsed);
                                // no updates, just wait for grace expiry
                            } else {
                                // Case 3: Long track (>40s) and played >40s
                                let base_entry = state.null_base_elapsed_by_session.entry(sid.clone())
                                    .or_insert_with(|| last_elapsed.saturating_sub(*NULL_GAP_REWIND_SECS));
                                let rewind_done = state.rewind_done_by_session.entry(sid.clone()).or_insert(false);
                                if !*rewind_done {
                                    *base_entry = last_elapsed.saturating_sub(*NULL_GAP_REWIND_SECS);
                                    *rewind_done = true;
                                    println!("⏪ Case 3: Rewind applied. Base set to {}s", *base_entry);
                                }
                                let display_elapsed = *base_entry + gap_secs as i64;

                                if last_discord_update.elapsed().as_secs() >= *DISCORD_UPDATE_INTERVAL_SECS {
                                    set_activity(&mut discord, &last_item, display_elapsed, rt_secs);
                                    state.active_session_id = Some(sid.clone());
                                    println!("✅ Case 3: Updated activity with elapsed={}s", display_elapsed);
                                    last_discord_update = Instant::now();
                                }
                            }

                            // === Expiry logic ===
                            if gap_secs > *NULL_GAP_MAX_SECS {
                                clear_discord(&mut discord);
                                state.active_session_id = None;
                                state.null_since_by_session.remove(&sid);
                                state.null_base_elapsed_by_session.remove(&sid);
                                state.rewind_done_by_session.remove(&sid);
                                state.last_item_by_session.remove(&sid);
                                state.last_pos_by_session.remove(&sid);
                                println!("🛑 Null gap > {}s: cleared activity", *NULL_GAP_MAX_SECS);
                                last_discord_update = Instant::now();
                            } else if last_elapsed >= rt_secs && rt_secs > 0 {
                                clear_discord(&mut discord);
                                state.active_session_id = None;
                                state.null_since_by_session.remove(&sid);
                                state.null_base_elapsed_by_session.remove(&sid);
                                state.rewind_done_by_session.remove(&sid);
                                state.last_item_by_session.remove(&sid);
                                state.last_pos_by_session.remove(&sid);
                                println!("🏁 End of item (elapsed {}s >= runtime {}s): cleared activity", last_elapsed, rt_secs);
                            }
                        } else if last_discord_update.elapsed().as_secs() >= *DISCORD_UPDATE_INTERVAL_SECS {
                            clear_discord(&mut discord);
                            state.active_session_id = None;
                            println!("❌ No last item found: cleared activity");
                            last_discord_update = Instant::now();
                        }
                    }
                } else if last_discord_update.elapsed().as_secs() >= *DISCORD_UPDATE_INTERVAL_SECS {
                    clear_discord(&mut discord);
                    state.active_session_id = None;
                    println!("❌ No active session for {}", USER.as_str());
                    last_discord_update = Instant::now();
                }
            }
        }

        thread::sleep(Duration::from_secs(*JELLYFIN_POLL_INTERVAL_SECS));
    }
}
