//! `spotify-tokens.json` beside the config file: `{"<account name>": "<refresh token>"}`.
//!
//! Spotify may return a new refresh token on each refresh ("When a refresh token is not
//! returned, continue using the existing token",
//! https://developer.spotify.com/documentation/web-api/tutorials/refreshing-tokens).
//! The config file is the user's and is never written by this app (the config watcher
//! would also restart it), so rotated tokens and tokens from `mmm-native auth spotify`
//! are kept here instead. For an account whose name has an entry here, the entry wins
//! over `components.spotify.accounts[].refreshToken`.
//!
//! The file is replaced atomically (temp file + rename) with mode 0600. Token values
//! are never logged.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const FILE: &str = "spotify-tokens.json";

pub type Saved = BTreeMap<String, String>;

/// The token file for a config file path.
pub fn path_for(config: &Path) -> PathBuf {
    let dir = config
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    dir.join(FILE)
}

/// The saved tokens; a missing file is an empty map. Errors carry no file contents.
pub fn load(path: &Path) -> Result<Saved, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str::<Saved>(&text)
            .map_err(|_| format!("{} is not a JSON object of strings", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Saved::new()),
        Err(e) => Err(format!("cannot read {}: {e}", path.display())),
    }
}

/// Store `token` for `name`, keeping the other entries. A file that exists but cannot
/// be parsed is not overwritten (it may hold tokens we could not read).
pub fn save(path: &Path, name: &str, token: &str) -> Result<(), String> {
    let mut saved = load(path)?;
    if saved.get(name).map(String::as_str) == Some(token) {
        return Ok(());
    }
    saved.insert(name.to_string(), token.to_string());
    let json = serde_json::to_string_pretty(&saved).map_err(|e| e.to_string())?;
    crate::google_auth::replace_file(path, json.as_bytes(), 0o600)
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// A configured account resolved against the saved tokens.
#[derive(Debug, PartialEq)]
pub struct Resolved {
    /// Position in `components.spotify.accounts`.
    pub index: usize,
    pub name: String,
    pub refresh_token: String,
    /// The token came from the file.
    pub from_file: bool,
}

/// Accounts from config as `(name, refreshToken)`, in config order. A saved token for a
/// non-empty name wins; otherwise the config token is used; accounts with neither are
/// left out.
pub fn resolve(accounts: &[(Option<String>, Option<String>)], saved: &Saved) -> Vec<Resolved> {
    accounts
        .iter()
        .enumerate()
        .filter_map(|(index, (name, token))| {
            let name = name.clone().unwrap_or_default();
            let from_file = (!name.is_empty())
                .then(|| saved.get(&name))
                .flatten()
                .filter(|t| !t.is_empty());
            let (refresh_token, from_file) = match from_file {
                Some(t) => (t.clone(), true),
                None => (token.clone().filter(|t| !t.is_empty())?, false),
            };
            Some(Resolved {
                index,
                name,
                refresh_token,
                from_file,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> TempDir {
            let n = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let p =
                std::env::temp_dir().join(format!("mmm-spotify-{tag}-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn acct(name: Option<&str>, token: Option<&str>) -> (Option<String>, Option<String>) {
        (name.map(str::to_string), token.map(str::to_string))
    }

    #[test]
    fn saved_token_wins_by_name() {
        let saved: Saved = [
            ("Alex".to_string(), "file-a".to_string()),
            ("Sam".into(), "".into()),
        ]
        .into();
        let accounts = [
            acct(Some("Alex"), Some("config-a")), // file wins
            acct(Some("Sam"), Some("config-s")),  // empty file entry: config
            acct(Some("Robin"), None),            // nothing anywhere: dropped
            acct(None, Some("config-unnamed")),   // no name: config only
            acct(Some(""), Some("config-blank")), // blank name never matches the file
        ];
        let saved2: Saved = [("".to_string(), "file-blank".to_string())].into();
        let r = resolve(&accounts, &saved);
        let got: Vec<_> = r
            .iter()
            .map(|a| {
                (
                    a.index,
                    a.name.as_str(),
                    a.refresh_token.as_str(),
                    a.from_file,
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                (0, "Alex", "file-a", true),
                (1, "Sam", "config-s", false),
                (3, "", "config-unnamed", false),
                (4, "", "config-blank", false),
            ]
        );
        assert!(!resolve(&accounts, &saved2)[3].from_file);
        // An account with only a name, authorised through the CLI.
        let only_file = resolve(
            &[acct(Some("Robin"), None)],
            &[("Robin".to_string(), "file-r".to_string())].into(),
        );
        assert_eq!(only_file[0].refresh_token, "file-r");
    }

    #[test]
    fn save_is_atomic_private_and_keeps_other_entries() {
        let dir = TempDir::new("save");
        let config = dir.0.join("config.json");
        let path = path_for(&config);
        assert_eq!(path, dir.0.join(FILE));
        assert_eq!(load(&path).unwrap(), Saved::new(), "missing file: empty");

        save(&path, "Alex", "t1").unwrap();
        save(&path, "Sam", "t2").unwrap();
        save(&path, "Alex", "t3").unwrap();
        let saved = load(&path).unwrap();
        assert_eq!(
            saved,
            [
                ("Alex".to_string(), "t3".to_string()),
                ("Sam".into(), "t2".into())
            ]
            .into()
        );
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        // Only the token file is left: no temp files.
        let names: Vec<_> = std::fs::read_dir(&dir.0)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, [std::ffi::OsString::from(FILE)]);

        // An unreadable (corrupt) file is not clobbered.
        std::fs::write(&path, "{not json").unwrap();
        let err = save(&path, "Alex", "secret-token").unwrap_err();
        assert!(
            !err.contains("secret-token") && !err.contains("not json"),
            "{err}"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{not json");
    }

    #[test]
    fn failed_write_leaves_the_old_file() {
        let dir = TempDir::new("ro");
        let path = dir.0.join(FILE);
        save(&path, "Alex", "t1").unwrap();
        std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(0o555)).unwrap();
        let result = save(&path, "Alex", "t2");
        std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(0o755)).unwrap();
        if result.is_ok() {
            return; // running as root: permissions are not enforced
        }
        assert_eq!(load(&path).unwrap()["Alex"], "t1");
    }
}
