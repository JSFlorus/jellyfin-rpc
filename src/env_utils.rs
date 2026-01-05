use std::{env, path::PathBuf};

pub fn load_local_env() {
    let exe_path = env::current_exe().expect("Failed to get exe path");
    let exe_dir: PathBuf = exe_path.parent().unwrap().to_path_buf();
    let env_file = exe_dir.join(".env");

    if env_file.exists() {
        dotenvy::from_filename_override(&env_file)
            .expect("Failed to load .env from exe directory");
        eprintln!("Loaded .env from {:?}", env_file);
    } else {
        eprintln!("No .env found in exe directory: {:?}", env_file);
    }
}
