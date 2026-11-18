use std::{fs, path::PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct AppSettings {
    pub width: f32,
    pub height: f32,
    pub last_tab: i32,
    pub maximized: bool,
    pub group_by_app: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            width: 1024.0,
            height: 680.0,
            last_tab: 0,
            maximized: false,
            group_by_app: true,
        }
    }
}

impl AppSettings {
    pub fn config_path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("system-monitor").join("settings.json")
    }

    pub fn parse_json(content: &str) -> Option<Self> {
        let mut settings = Self::default();
        let mut found_any = false;

        for line in content.lines() {
            let trimmed = line.trim().trim_end_matches(',');
            if let Some(rest) = trimmed.strip_prefix("\"width\":") {
                if let Ok(w) = rest.trim().parse::<f32>() {
                    if (400.0..=8000.0).contains(&w) {
                        settings.width = w;
                        found_any = true;
                    }
                }
            } else if let Some(rest) = trimmed.strip_prefix("\"height\":") {
                if let Ok(h) = rest.trim().parse::<f32>() {
                    if (300.0..=6000.0).contains(&h) {
                        settings.height = h;
                        found_any = true;
                    }
                }
            } else if let Some(rest) = trimmed.strip_prefix("\"last_tab\":") {
                if let Ok(tab) = rest.trim().parse::<i32>() {
                    if (0..=2).contains(&tab) {
                        settings.last_tab = tab;
                        found_any = true;
                    }
                }
            } else if let Some(rest) = trimmed.strip_prefix("\"maximized\":") {
                if let Ok(max) = rest.trim().parse::<bool>() {
                    settings.maximized = max;
                    found_any = true;
                }
            } else if let Some(rest) = trimmed.strip_prefix("\"group_by_app\":") {
                if let Ok(grp) = rest.trim().parse::<bool>() {
                    settings.group_by_app = grp;
                    found_any = true;
                }
            }
        }

        if found_any {
            Some(settings)
        } else {
            None
        }
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\n  \"width\": {:.1},\n  \"height\": {:.1},\n  \"last_tab\": {},\n  \"maximized\": {},\n  \"group_by_app\": {}\n}}\n",
            self.width, self.height, self.last_tab, self.maximized, self.group_by_app
        )
    }

    pub fn load() -> Self {
        Self::load_from(&Self::config_path())
    }

    pub fn load_from(path: &std::path::Path) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => Self::parse_json(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        self.save_to(&Self::config_path())
    }

    pub fn save_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, self.to_json())?;
        fs::rename(&temp_path, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings() {
        let s = AppSettings::default();
        assert_eq!(s.width, 1024.0);
        assert_eq!(s.height, 680.0);
        assert_eq!(s.last_tab, 0);
        assert!(!s.maximized);
        assert!(s.group_by_app);
    }

    #[test]
    fn parse_and_serialize_roundtrip() {
        let original = AppSettings {
            width: 1280.0,
            height: 720.0,
            last_tab: 2,
            maximized: true,
            group_by_app: false,
        };
        let json = original.to_json();
        let parsed = AppSettings::parse_json(&json).expect("should parse json");
        assert_eq!(parsed, original);
    }

    #[test]
    fn parse_invalid_json_fallback() {
        assert!(AppSettings::parse_json("invalid json").is_none());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("sysmon_test_{}", std::process::id()));
        let file = dir.join("settings.json");

        let expected = AppSettings {
            width: 1100.0,
            height: 750.0,
            last_tab: 1,
            maximized: false,
            group_by_app: true,
        };
        expected.save_to(&file).expect("save should succeed");
        let loaded = AppSettings::load_from(&file);
        assert_eq!(loaded, expected);

        let _ = fs::remove_dir_all(&dir);
    }
}
