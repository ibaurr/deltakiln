use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{Error, Result};

/// Name of the config file looked up in the site root.
pub const CONFIG_FILE: &str = "deltakiln.toml";

/// Site configuration. Every field is optional; a site with no
/// `deltakiln.toml` at all builds with these defaults.
///
/// Directory fields are relative to the site root.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub title: String,
    pub base_url: String,
    pub content_dir: PathBuf,
    pub templates_dir: PathBuf,
    pub static_dir: PathBuf,
    pub output_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            title: "My Site".to_string(),
            base_url: String::new(),
            content_dir: PathBuf::from("content"),
            templates_dir: PathBuf::from("templates"),
            static_dir: PathBuf::from("static"),
            output_dir: PathBuf::from("public"),
        }
    }
}

impl Config {
    /// Load `<root>/deltakiln.toml`, falling back to defaults when absent.
    pub fn load(root: &Path) -> Result<Config> {
        let path = root.join(CONFIG_FILE);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
            Err(e) => return Err(Error::io(path, e)),
        };
        // Same normalisation as sources: a CRLF checkout must behave like LF.
        let text = text.replace("\r\n", "\n");
        toml::from_str(&text).map_err(|e| Error::Config {
            path,
            message: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempdir();
        assert_eq!(Config::load(&dir).unwrap(), Config::default());
    }

    #[test]
    fn partial_file_overrides_only_given_fields() {
        let dir = tempdir();
        std::fs::write(dir.join(CONFIG_FILE), "title = \"Docs\"\n").unwrap();
        let cfg = Config::load(&dir).unwrap();
        assert_eq!(cfg.title, "Docs");
        assert_eq!(cfg.content_dir, PathBuf::from("content"));
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let dir = tempdir();
        std::fs::write(dir.join(CONFIG_FILE), "titel = \"typo\"\n").unwrap();
        let err = Config::load(&dir).unwrap_err().to_string();
        assert!(err.contains("titel"), "{err}");
    }

    #[test]
    fn crlf_config_parses() {
        let dir = tempdir();
        std::fs::write(
            dir.join(CONFIG_FILE),
            "title = \"A\"\r\nbase_url = \"/x\"\r\n",
        )
        .unwrap();
        let cfg = Config::load(&dir).unwrap();
        assert_eq!((cfg.title.as_str(), cfg.base_url.as_str()), ("A", "/x"));
    }

    /// Unique scratch dir without pulling in a dev-dependency for this crate.
    fn tempdir() -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "deltakiln-core-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
