use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use toml_edit::DocumentMut;

const PLUGIN_ID: &str = "herdr-scratch-pane";
const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginConfig {
    pub mark_workspace: bool,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            mark_workspace: true,
        }
    }
}

pub fn load_plugin_config() -> Result<PluginConfig> {
    let Some(path) = plugin_config_path() else {
        return Ok(PluginConfig::default());
    };
    load_plugin_config_from(&path)
}

fn load_plugin_config_from(path: &Path) -> Result<PluginConfig> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PluginConfig::default())
        }
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()))
        }
    };

    parse_plugin_config(&text)
        .with_context(|| format!("failed to parse plugin config {}", path.display()))
}

fn parse_plugin_config(text: &str) -> Result<PluginConfig> {
    let document = text.parse::<DocumentMut>()?;
    let Some(value) = document.get("mark_workspace") else {
        return Ok(PluginConfig::default());
    };
    let Some(mark_workspace) = value.as_bool() else {
        bail!("`mark_workspace` must be a boolean");
    };
    Ok(PluginConfig { mark_workspace })
}

fn plugin_config_path() -> Option<PathBuf> {
    plugin_config_path_from(
        std::env::var_os("HERDR_PLUGIN_CONFIG_DIR").as_deref(),
        std::env::var_os("XDG_CONFIG_HOME").as_deref(),
        std::env::var_os("HOME").as_deref(),
    )
}

fn plugin_config_path_from(
    plugin_config_dir: Option<&OsStr>,
    xdg_config_home: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Option<PathBuf> {
    if let Some(dir) = plugin_config_dir {
        return Some(PathBuf::from(dir).join(CONFIG_FILE));
    }

    let config_root = xdg_config_home
        .map(PathBuf::from)
        .or_else(|| home.map(|home| PathBuf::from(home).join(".config")))?;
    Some(
        config_root
            .join("herdr/plugins/config")
            .join(PLUGIN_ID)
            .join(CONFIG_FILE),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_uses_backward_compatible_default() {
        assert_eq!(parse_plugin_config("").unwrap(), PluginConfig::default());
        assert!(PluginConfig::default().mark_workspace);
    }

    #[test]
    fn parses_explicit_workspace_marker_setting() {
        assert!(
            parse_plugin_config("mark_workspace = true\n")
                .unwrap()
                .mark_workspace
        );
        assert!(
            !parse_plugin_config("mark_workspace = false\n")
                .unwrap()
                .mark_workspace
        );
    }

    #[test]
    fn rejects_invalid_workspace_marker_setting() {
        let error = parse_plugin_config("mark_workspace = \"no\"\n").unwrap_err();
        assert!(error.to_string().contains("must be a boolean"));
        assert!(parse_plugin_config("mark_workspace = [\n").is_err());
    }

    #[test]
    fn missing_file_uses_default() {
        let path = std::env::temp_dir().join(format!(
            "herdr-scratch-pane-missing-config-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        assert_eq!(
            load_plugin_config_from(&path).unwrap(),
            PluginConfig::default()
        );
    }

    #[test]
    fn config_path_prefers_injected_plugin_directory() {
        assert_eq!(
            plugin_config_path_from(
                Some(OsStr::new("/plugin-config")),
                Some(OsStr::new("/xdg")),
                Some(OsStr::new("/home/me")),
            ),
            Some(PathBuf::from("/plugin-config/config.toml"))
        );
    }

    #[test]
    fn config_path_falls_back_to_herdr_xdg_location() {
        assert_eq!(
            plugin_config_path_from(None, Some(OsStr::new("/xdg")), Some(OsStr::new("/home/me")),),
            Some(PathBuf::from(
                "/xdg/herdr/plugins/config/herdr-scratch-pane/config.toml"
            ))
        );
        assert_eq!(
            plugin_config_path_from(None, None, Some(OsStr::new("/home/me"))),
            Some(PathBuf::from(
                "/home/me/.config/herdr/plugins/config/herdr-scratch-pane/config.toml"
            ))
        );
    }
}
