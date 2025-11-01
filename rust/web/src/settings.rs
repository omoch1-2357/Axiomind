use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use thiserror::Error;

/// Application settings that can be configured through the web interface
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    /// Default blind level (1-10)
    pub default_level: u8,
    /// Default AI difficulty/strategy name
    pub default_ai_strategy: String,
    /// Session timeout in minutes
    pub session_timeout_minutes: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_level: 1,
            default_ai_strategy: "baseline".to_string(),
            session_timeout_minutes: 30,
        }
    }
}

impl AppSettings {
    /// Validate settings values
    pub fn validate(&self) -> Result<(), SettingsError> {
        if self.default_level < 1 || self.default_level > 10 {
            return Err(SettingsError::InvalidValue(
                "default_level must be between 1 and 10".to_string(),
            ));
        }

        if self.default_ai_strategy.is_empty() {
            return Err(SettingsError::InvalidValue(
                "default_ai_strategy cannot be empty".to_string(),
            ));
        }

        if self.session_timeout_minutes == 0 {
            return Err(SettingsError::InvalidValue(
                "session_timeout_minutes must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

/// In-memory settings store with validation
#[derive(Debug)]
pub struct SettingsStore {
    settings: RwLock<AppSettings>,
}

impl SettingsStore {
    pub fn new() -> Self {
        Self {
            settings: RwLock::new(AppSettings::default()),
        }
    }

    pub fn with_settings(settings: AppSettings) -> Result<Self, SettingsError> {
        settings.validate()?;
        Ok(Self {
            settings: RwLock::new(settings),
        })
    }

    /// Get current settings
    pub fn get(&self) -> Result<AppSettings, SettingsError> {
        self.settings
            .read()
            .map(|guard| guard.clone())
            .map_err(|_| SettingsError::StoragePoisoned)
    }

    /// Update settings with validation
    pub fn update(&self, new_settings: AppSettings) -> Result<AppSettings, SettingsError> {
        new_settings.validate()?;

        let mut guard = self
            .settings
            .write()
            .map_err(|_| SettingsError::StoragePoisoned)?;
        *guard = new_settings.clone();
        Ok(new_settings)
    }

    /// Update specific field
    pub fn update_field(
        &self,
        field: &str,
        value: serde_json::Value,
    ) -> Result<AppSettings, SettingsError> {
        let mut current = self.get()?;

        match field {
            "default_level" => {
                let level = value.as_u64().ok_or_else(|| {
                    SettingsError::InvalidValue("default_level must be a number".to_string())
                })? as u8;
                current.default_level = level;
            }
            "default_ai_strategy" => {
                let strategy = value.as_str().ok_or_else(|| {
                    SettingsError::InvalidValue("default_ai_strategy must be a string".to_string())
                })?;
                current.default_ai_strategy = strategy.to_string();
            }
            "session_timeout_minutes" => {
                let timeout = value.as_u64().ok_or_else(|| {
                    SettingsError::InvalidValue(
                        "session_timeout_minutes must be a number".to_string(),
                    )
                })?;
                current.session_timeout_minutes = timeout;
            }
            _ => {
                return Err(SettingsError::InvalidValue(format!(
                    "unknown field: {}",
                    field
                )))
            }
        }

        self.update(current)
    }

    /// Reset to default settings
    pub fn reset(&self) -> Result<AppSettings, SettingsError> {
        let defaults = AppSettings::default();
        self.update(defaults)
    }
}

impl Default for SettingsStore {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("Invalid settings value: {0}")]
    InvalidValue(String),
    #[error("Settings storage poisoned")]
    StoragePoisoned,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_valid() {
        let settings = AppSettings::default();
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn validates_blind_level_range() {
        let settings = AppSettings {
            default_level: 0,
            ..Default::default()
        };
        assert!(settings.validate().is_err());

        let settings = AppSettings {
            default_level: 11,
            ..Default::default()
        };
        assert!(settings.validate().is_err());

        let settings = AppSettings {
            default_level: 1,
            ..Default::default()
        };
        assert!(settings.validate().is_ok());

        let settings = AppSettings {
            default_level: 10,
            ..Default::default()
        };
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn validates_ai_strategy_not_empty() {
        let settings = AppSettings {
            default_ai_strategy: "".to_string(),
            ..Default::default()
        };
        assert!(settings.validate().is_err());

        let settings = AppSettings {
            default_ai_strategy: "baseline".to_string(),
            ..Default::default()
        };
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn validates_session_timeout_positive() {
        let settings = AppSettings {
            session_timeout_minutes: 0,
            ..Default::default()
        };
        assert!(settings.validate().is_err());

        let settings = AppSettings {
            session_timeout_minutes: 1,
            ..Default::default()
        };
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn settings_store_provides_defaults() {
        let store = SettingsStore::new();
        let settings = store.get().expect("get settings");
        assert_eq!(settings, AppSettings::default());
    }

    #[test]
    fn settings_store_updates_with_validation() {
        let store = SettingsStore::new();

        let new_settings = AppSettings {
            default_level: 5,
            default_ai_strategy: "aggressive".to_string(),
            ..Default::default()
        };

        let updated = store.update(new_settings.clone()).expect("update");
        assert_eq!(updated, new_settings);

        let retrieved = store.get().expect("get");
        assert_eq!(retrieved, new_settings);
    }

    #[test]
    fn settings_store_rejects_invalid_updates() {
        let store = SettingsStore::new();

        let invalid = AppSettings {
            default_level: 99,
            ..Default::default()
        };

        assert!(store.update(invalid).is_err());

        // Original settings unchanged
        let current = store.get().expect("get");
        assert_eq!(current.default_level, 1);
    }

    #[test]
    fn settings_store_updates_individual_fields() {
        let store = SettingsStore::new();

        store
            .update_field("default_level", serde_json::json!(3))
            .expect("update level");
        let settings = store.get().expect("get");
        assert_eq!(settings.default_level, 3);

        store
            .update_field("default_ai_strategy", serde_json::json!("aggressive"))
            .expect("update strategy");
        let settings = store.get().expect("get");
        assert_eq!(settings.default_ai_strategy, "aggressive");

        store
            .update_field("session_timeout_minutes", serde_json::json!(60))
            .expect("update timeout");
        let settings = store.get().expect("get");
        assert_eq!(settings.session_timeout_minutes, 60);
    }

    #[test]
    fn settings_store_validates_field_updates() {
        let store = SettingsStore::new();

        // Invalid level
        assert!(store
            .update_field("default_level", serde_json::json!(99))
            .is_err());

        // Invalid type
        assert!(store
            .update_field("default_level", serde_json::json!("not a number"))
            .is_err());

        // Unknown field
        assert!(store
            .update_field("unknown_field", serde_json::json!(42))
            .is_err());

        // Settings remain unchanged after failed updates
        let current = store.get().expect("get");
        assert_eq!(current, AppSettings::default());
    }

    #[test]
    fn settings_store_resets_to_defaults() {
        let store = SettingsStore::new();

        let custom = AppSettings {
            default_level: 5,
            default_ai_strategy: "custom".to_string(),
            ..Default::default()
        };
        store.update(custom).expect("update");

        let reset = store.reset().expect("reset");
        assert_eq!(reset, AppSettings::default());

        let current = store.get().expect("get");
        assert_eq!(current, AppSettings::default());
    }

    #[test]
    fn settings_store_thread_safe() {
        use std::sync::Arc;
        use std::thread;

        let store = Arc::new(SettingsStore::new());
        let mut handles = Vec::new();

        for i in 1..=5 {
            let store = Arc::clone(&store);
            handles.push(thread::spawn(move || {
                let settings = AppSettings {
                    default_level: i,
                    default_ai_strategy: format!("strategy_{}", i),
                    ..Default::default()
                };
                store.update(settings).ok();
            }));
        }

        for handle in handles {
            handle.join().expect("join thread");
        }

        // Final state should be valid
        let final_settings = store.get().expect("get");
        assert!(final_settings.validate().is_ok());
    }
}
