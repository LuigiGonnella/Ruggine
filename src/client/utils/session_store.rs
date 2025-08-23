use keyring::Entry;

const SERVICE: &str = "ruggine_app";
const USER: &str = "ruggine_session";

pub fn save_session_token(token: &str) -> anyhow::Result<()> {
    let entry = Entry::new(SERVICE, USER);
    match entry.set_password(token) {
        Ok(()) => {
            // token stored securely in OS keyring
            Ok(())
        }
    Err(_e) => {
            // Keyring failed. Optionally fall back to a local file when explicitly allowed
            let allow_fallback = std::env::var("KEYRING_FALLBACK").unwrap_or_default() == "true";
            if allow_fallback {
                let path = std::path::Path::new("data").join("session_token.txt");
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                std::fs::write(&path, token)?;
                // warn in logs but do not print token
                println!("[SESSION_STORE] Keyring unavailable, persisted token to fallback file");
                Ok(())
            } else {
                // do not persist to disk silently; return error so caller can decide
                Err(anyhow::anyhow!("keyring unavailable and file fallback disabled"))
            }
        }
    }
}

pub fn load_session_token() -> Option<String> {
    let entry = Entry::new(SERVICE, USER);
    match entry.get_password() {
        Ok(t) => {
            if t.trim().is_empty() { None } else { Some(t) }
        }
        Err(_e) => {
            // Only attempt file fallback when explicitly enabled via env var
            let allow_fallback = std::env::var("KEYRING_FALLBACK").unwrap_or_default() == "true";
            if allow_fallback {
                let path = std::path::Path::new("data").join("session_token.txt");
                if path.exists() {
                    if let Ok(s) = std::fs::read_to_string(&path) {
                        let t = s.trim().to_string();
                        if !t.is_empty() {
                            return Some(t);
                        }
                    }
                }
            }
            None
        }
    }
}

pub fn clear_session_token() -> anyhow::Result<()> {
    let entry = Entry::new(SERVICE, USER);
    let _ = entry.delete_password();
    // remove fallback file only if fallback is enabled
    let allow_fallback = std::env::var("KEYRING_FALLBACK").unwrap_or_default() == "true";
    if allow_fallback {
        let path = std::path::Path::new("data").join("session_token.txt");
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
    }
    Ok(())
}
