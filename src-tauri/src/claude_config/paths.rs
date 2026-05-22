use std::path::PathBuf;

/// The two Claude surfaces this app knows how to wire up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeSurface {
    Code,
    Desktop,
}

impl ClaudeSurface {
    pub fn display_name(self) -> &'static str {
        match self {
            ClaudeSurface::Code => "Claude Code",
            ClaudeSurface::Desktop => "Claude Desktop",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("could not resolve home directory")]
    NoHome,
    #[error("platform not supported for Claude {0}")]
    UnsupportedPlatform(&'static str),
}

/// Returns the canonical Claude Code config path for the current OS.
/// - macOS / Linux: `~/.claude.json`
/// - Windows:      `%USERPROFILE%\.claude.json`
pub fn claude_code_config_path() -> Result<PathBuf, PathError> {
    let home = dirs::home_dir().ok_or(PathError::NoHome)?;
    Ok(home.join(".claude.json"))
}

/// Returns the canonical Claude Desktop config path for the current OS.
/// - macOS:   `~/Library/Application Support/Claude/claude_desktop_config.json`
/// - Linux:   `~/.config/Claude/claude_desktop_config.json`  (per Claude Desktop's published location)
/// - Windows: `%APPDATA%\Claude\claude_desktop_config.json`
pub fn claude_desktop_config_path() -> Result<PathBuf, PathError> {
    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().ok_or(PathError::NoHome)?;
        Ok(home
            .join("Library")
            .join("Application Support")
            .join("Claude")
            .join("claude_desktop_config.json"))
    }
    #[cfg(target_os = "linux")]
    {
        let cfg = dirs::config_dir().ok_or(PathError::NoHome)?;
        Ok(cfg.join("Claude").join("claude_desktop_config.json"))
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = dirs::config_dir().ok_or(PathError::NoHome)?;
        Ok(appdata.join("Claude").join("claude_desktop_config.json"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err(PathError::UnsupportedPlatform("Desktop"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_display_names_are_user_facing() {
        assert_eq!(ClaudeSurface::Code.display_name(), "Claude Code");
        assert_eq!(ClaudeSurface::Desktop.display_name(), "Claude Desktop");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn claude_code_path_on_macos_lives_in_home() {
        let path = claude_code_config_path().unwrap();
        let s = path.to_string_lossy();
        assert!(s.ends_with("/.claude.json"), "got {s}");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn claude_desktop_path_on_macos_lives_in_app_support() {
        let path = claude_desktop_config_path().unwrap();
        let s = path.to_string_lossy();
        assert!(
            s.contains("/Library/Application Support/Claude/claude_desktop_config.json"),
            "got {s}"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn claude_code_path_on_linux_lives_in_home() {
        let path = claude_code_config_path().unwrap();
        let s = path.to_string_lossy();
        assert!(s.ends_with("/.claude.json"), "got {s}");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn claude_code_path_on_windows_uses_userprofile() {
        let path = claude_code_config_path().unwrap();
        let s = path.to_string_lossy();
        assert!(s.ends_with("\\.claude.json"), "got {s}");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn claude_desktop_path_on_windows_uses_appdata() {
        let path = claude_desktop_config_path().unwrap();
        let s = path.to_string_lossy();
        assert!(
            s.contains("\\Claude\\claude_desktop_config.json"),
            "got {s}"
        );
    }
}
