use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
pub struct DevcapConfig {
    pub path: Option<PathBuf>,
    pub author: Option<String>,
    pub period: Option<String>,
    pub show_origin: Option<bool>,
    pub color: Option<bool>,
}

pub fn load() -> DevcapConfig {
    match try_load() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Warning: failed to load ~/.devcap.toml: {e}");
            DevcapConfig::default()
        }
    }
}

fn try_load() -> Result<DevcapConfig> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("HOME not set"))?;
    let config_path = home.join(".devcap.toml");
    if !config_path.exists() {
        return Ok(DevcapConfig::default());
    }
    let content = std::fs::read_to_string(&config_path)?;
    let mut config: DevcapConfig = toml::from_str(&content)?;
    config.path = config.path.map(|p| expand_tilde(p, &home));
    Ok(config)
}

fn expand_tilde(path: PathBuf, home: &std::path::Path) -> PathBuf {
    if let Ok(rest) = path.strip_prefix("~") {
        home.join(rest)
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_all_none() {
        let cfg = DevcapConfig::default();
        assert!(cfg.path.is_none());
        assert!(cfg.author.is_none());
        assert!(cfg.period.is_none());
        assert!(cfg.show_origin.is_none());
        assert!(cfg.color.is_none());
    }

    #[test]
    fn parse_partial_config() {
        let toml_str = r#"
            author = "Jane Doe"
            period = "week"
        "#;
        let cfg: DevcapConfig = toml::from_str(toml_str).expect("parse failed");
        assert_eq!(cfg.author.as_deref(), Some("Jane Doe"));
        assert_eq!(cfg.period.as_deref(), Some("week"));
        assert!(cfg.path.is_none());
    }

    #[test]
    fn parse_full_config() {
        let toml_str = r#"
            path = "/home/user/projects"
            author = "John"
            period = "7d"
            show_origin = true
            color = false
        "#;
        let cfg: DevcapConfig = toml::from_str(toml_str).expect("parse failed");
        assert_eq!(cfg.path, Some(PathBuf::from("/home/user/projects")));
        assert_eq!(cfg.author.as_deref(), Some("John"));
        assert_eq!(cfg.period.as_deref(), Some("7d"));
        assert_eq!(cfg.show_origin, Some(true));
        assert_eq!(cfg.color, Some(false));
    }

    #[test]
    fn tilde_path_is_expanded() {
        let home = PathBuf::from("/home/user");
        let result = expand_tilde(PathBuf::from("~/Sites"), &home);
        assert_eq!(result, PathBuf::from("/home/user/Sites"));
    }

    #[test]
    fn absolute_path_is_unchanged() {
        let home = PathBuf::from("/home/user");
        let result = expand_tilde(PathBuf::from("/opt/repos"), &home);
        assert_eq!(result, PathBuf::from("/opt/repos"));
    }

    #[test]
    fn tilde_only_expands_to_home() {
        let home = PathBuf::from("/home/user");
        let result = expand_tilde(PathBuf::from("~"), &home);
        assert_eq!(result, PathBuf::from("/home/user"));
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let toml_str = r#"
            author = "Test"
            unknown_field = "should not fail"
        "#;
        // toml by default errors on unknown fields with deny_unknown_fields,
        // but our struct does not have that attribute, so this should parse.
        let result: Result<DevcapConfig, _> = toml::from_str(toml_str);
        assert!(result.is_ok());
    }
}
