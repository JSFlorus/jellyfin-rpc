use std::{
    env,
    path::PathBuf,
};

/// Load a `.env` file located next to the compiled binary.
/// This allows the binary to be fully self-contained.
pub fn load_local_env() {
    let exe_path = env::current_exe().expect("Failed to get exe path");
    let exe_dir: PathBuf = exe_path
        .parent()
        .expect("Failed to get exe dir")
        .to_path_buf();

    let env_file = exe_dir.join(".env");

    if env_file.exists() {
        dotenvy::from_filename_override(&env_file)
            .expect("Failed to load .env from exe directory");
        eprintln!("Loaded .env from {:?}", env_file);
    } else {
        eprintln!("No .env found in exe directory: {:?}", env_file);
    }
}

/// Configure Discord IPC.
///
/// Linux Discord RPC libs usually expect:
///   $XDG_RUNTIME_DIR/discord-ipc-0
///
/// Supported configuration via `.env`:
///   • XDG_RUNTIME_DIR  (you can set this directly)
///   • DISCORD_IPC_DIR  (copied into XDG_RUNTIME_DIR)
///
/// On non-Linux platforms, this does nothing so the crate can use its defaults (e.g. Windows pipes).
pub fn configure_discord_ipc_env() {
    // Non-Linux: do nothing and let the Discord RPC crate handle defaults.
    #[cfg(not(target_os = "linux"))]
    {
        return;
    }

    // Linux implementation
    #[cfg(target_os = "linux")]
    {
        use std::{
            env,
            fs,
            path::{Path, PathBuf},
        };
        use std::os::unix::fs::FileTypeExt;

        fn has_discord_socket(dir: &Path) -> bool {
            let socket = dir.join("discord-ipc-0");
            match fs::metadata(&socket) {
                Ok(meta) => meta.file_type().is_socket(),
                Err(_) => false,
            }
        }
        // 1) If DISCORD_IPC_DIR is set (from .env), use it.
        if let Ok(dir) = env::var("DISCORD_IPC_DIR") {
            env::set_var("XDG_RUNTIME_DIR", &dir);
            eprintln!("Discord IPC: using DISCORD_IPC_DIR -> XDG_RUNTIME_DIR = {}", dir);
            return;
        }
        // 2) If XDG_RUNTIME_DIR is set AND valid, keep it.
        // If it's set-but-wrong (common in containers), we keep trying fallbacks.
        if let Ok(xdg) = env::var("XDG_RUNTIME_DIR") {
            let p = PathBuf::from(&xdg);
            if has_discord_socket(&p) {
                eprintln!("Discord IPC: using existing XDG_RUNTIME_DIR ({})", xdg);
                return;
            }
            eprintln!(
                "Discord IPC: XDG_RUNTIME_DIR set but no discord-ipc-0 found ({}), trying fallbacks...",
                xdg
            );
        }
        // 3) Try common Flatpak paths under /run/user/<uid>
        // (requires libc = "0.2" in Cargo.toml)
        let uid = unsafe { libc::geteuid() };
        let base = PathBuf::from(format!("/run/user/{}", uid));

        let candidates = [
            base.join("app/com.discordapp.Discord"),
            base.join("app/com.discordapp.DiscordCanary"),
            base.join("app/com.discordapp.DiscordPTB"),
        ];

        for c in candidates {
            if has_discord_socket(&c) {
                let s = c.to_string_lossy().to_string();
                env::set_var("XDG_RUNTIME_DIR", &s);
                eprintln!("Discord IPC: detected Flatpak socket, set XDG_RUNTIME_DIR = {}", s);
                return;
            }
        }

        // 4) Last resort: scan /run/user/*
        let run_user = Path::new("/run/user");
        if let Ok(entries) = fs::read_dir(run_user) {
            for entry in entries.flatten() {
                let dir = entry.path();
                if !dir.is_dir() {
                    continue;
                }

                if has_discord_socket(&dir) {
                    let s = dir.to_string_lossy().to_string();
                    env::set_var("XDG_RUNTIME_DIR", &s);
                    eprintln!("Discord IPC: found socket, set XDG_RUNTIME_DIR = {}", s);
                    return;
                }

                let subs = [
                    dir.join("app/com.discordapp.Discord"),
                    dir.join("app/com.discordapp.DiscordCanary"),
                    dir.join("app/com.discordapp.DiscordPTB"),
                ];

                for c in subs {
                    if has_discord_socket(&c) {
                        let s = c.to_string_lossy().to_string();
                        env::set_var("XDG_RUNTIME_DIR", &s);
                        eprintln!("Discord IPC: found Flatpak socket, set XDG_RUNTIME_DIR = {}", s);
                        return;
                    }
                }
            }
        }

        eprintln!("Discord IPC: could not find discord-ipc-0 (is Discord running?)");
    }
}
