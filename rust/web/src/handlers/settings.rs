use crate::settings::{SettingsError, SettingsStore};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use warp::http::StatusCode;
use warp::reply::{self, Response};
use warp::Reply;

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub default_level: Option<u8>,
    pub default_ai_strategy: Option<String>,
    pub session_timeout_minutes: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFieldRequest {
    pub field: String,
    pub value: serde_json::Value,
}

/// Get current settings
pub async fn get_settings(store: Arc<SettingsStore>) -> Response {
    match store.get() {
        Ok(settings) => success_response(StatusCode::OK, settings),
        Err(err) => settings_error(err),
    }
}

/// Update settings
pub async fn update_settings(
    store: Arc<SettingsStore>,
    request: UpdateSettingsRequest,
) -> Response {
    let mut current = match store.get() {
        Ok(s) => s,
        Err(err) => return settings_error(err),
    };

    if let Some(level) = request.default_level {
        current.default_level = level;
    }

    if let Some(strategy) = request.default_ai_strategy {
        current.default_ai_strategy = strategy;
    }

    if let Some(timeout) = request.session_timeout_minutes {
        current.session_timeout_minutes = timeout;
    }

    match store.update(current) {
        Ok(settings) => success_response(StatusCode::OK, settings),
        Err(err) => settings_error(err),
    }
}

/// Update a single field
pub async fn update_field(store: Arc<SettingsStore>, request: UpdateFieldRequest) -> Response {
    match store.update_field(&request.field, request.value) {
        Ok(settings) => success_response(StatusCode::OK, settings),
        Err(err) => settings_error(err),
    }
}

/// Reset settings to defaults
pub async fn reset_settings(store: Arc<SettingsStore>) -> Response {
    match store.reset() {
        Ok(settings) => success_response(StatusCode::OK, settings),
        Err(err) => settings_error(err),
    }
}

fn success_response<T>(status: StatusCode, body: T) -> Response
where
    T: Serialize,
{
    reply::with_status(reply::json(&body), status).into_response()
}

fn settings_error(err: SettingsError) -> Response {
    use crate::errors::IntoErrorResponse;
    err.into_http_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppSettings;
    use std::sync::Arc;

    #[tokio::test]
    async fn get_settings_returns_current_settings() {
        let store = Arc::new(SettingsStore::new());
        let response = get_settings(store).await;

        // Response should be OK with default settings
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn update_settings_modifies_values() {
        let store = Arc::new(SettingsStore::new());

        let request = UpdateSettingsRequest {
            default_level: Some(5),
            default_ai_strategy: Some("aggressive".to_string()),
            session_timeout_minutes: Some(60),
        };

        let response = update_settings(store.clone(), request).await;
        assert_eq!(response.status(), StatusCode::OK);

        let settings = store.get().expect("get settings");
        assert_eq!(settings.default_level, 5);
        assert_eq!(settings.default_ai_strategy, "aggressive");
        assert_eq!(settings.session_timeout_minutes, 60);
    }

    #[tokio::test]
    async fn update_settings_validates_input() {
        let store = Arc::new(SettingsStore::new());

        let request = UpdateSettingsRequest {
            default_level: Some(99), // Invalid
            default_ai_strategy: None,
            session_timeout_minutes: None,
        };

        let response = update_settings(store.clone(), request).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Settings should remain unchanged
        let settings = store.get().expect("get settings");
        assert_eq!(settings.default_level, 1);
    }

    #[tokio::test]
    async fn update_field_changes_individual_field() {
        let store = Arc::new(SettingsStore::new());

        let request = UpdateFieldRequest {
            field: "default_level".to_string(),
            value: serde_json::json!(3),
        };

        let response = update_field(store.clone(), request).await;
        assert_eq!(response.status(), StatusCode::OK);

        let settings = store.get().expect("get settings");
        assert_eq!(settings.default_level, 3);
    }

    #[tokio::test]
    async fn update_field_validates_value() {
        let store = Arc::new(SettingsStore::new());

        let request = UpdateFieldRequest {
            field: "default_level".to_string(),
            value: serde_json::json!(99),
        };

        let response = update_field(store.clone(), request).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn reset_settings_restores_defaults() {
        let store = Arc::new(SettingsStore::new());

        // Modify settings
        let custom = AppSettings {
            default_level: 5,
            default_ai_strategy: "custom".to_string(),
            session_timeout_minutes: 120,
        };
        store.update(custom).expect("update");

        // Reset
        let response = reset_settings(store.clone()).await;
        assert_eq!(response.status(), StatusCode::OK);

        let settings = store.get().expect("get settings");
        assert_eq!(settings, AppSettings::default());
    }

    #[tokio::test]
    async fn partial_update_preserves_other_fields() {
        let store = Arc::new(SettingsStore::new());

        // Set initial state
        let initial = AppSettings {
            default_level: 3,
            default_ai_strategy: "aggressive".to_string(),
            session_timeout_minutes: 45,
        };
        store.update(initial).expect("update");

        // Update only one field
        let request = UpdateSettingsRequest {
            default_level: Some(7),
            default_ai_strategy: None,
            session_timeout_minutes: None,
        };

        update_settings(store.clone(), request).await;

        let settings = store.get().expect("get settings");
        assert_eq!(settings.default_level, 7);
        assert_eq!(settings.default_ai_strategy, "aggressive"); // Unchanged
        assert_eq!(settings.session_timeout_minutes, 45); // Unchanged
    }
}
