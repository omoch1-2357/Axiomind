use std::time::Instant;
use warp::http::StatusCode;
use warp::reject::Rejection;
use warp::reply::Reply;
use warp::Filter;

/// Middleware for logging HTTP requests and responses
pub fn with_request_logging<F, T>(
    filter: F,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone
where
    F: Filter<Extract = (T,), Error = Rejection> + Clone + Send + Sync + 'static,
    T: Reply,
{
    warp::any()
        .and(warp::path::full())
        .and(warp::method())
        .map(|path: warp::path::FullPath, method: warp::http::Method| {
            let start = Instant::now();
            tracing::info!(
                path = %path.as_str(),
                method = %method,
                "incoming request"
            );
            start
        })
        .and(filter)
        .map(|start: Instant, reply: T| {
            let duration = start.elapsed();
            tracing::info!(duration_ms = duration.as_millis(), "request completed");
            reply
        })
}

/// Log response with status code
pub fn log_response(status: StatusCode, path: &str, method: &str, duration_ms: u128) {
    if status.is_success() {
        tracing::info!(
            status = %status.as_u16(),
            path = %path,
            method = %method,
            duration_ms = duration_ms,
            "response sent"
        );
    } else if status.is_client_error() {
        tracing::warn!(
            status = %status.as_u16(),
            path = %path,
            method = %method,
            duration_ms = duration_ms,
            "client error"
        );
    } else if status.is_server_error() {
        tracing::error!(
            status = %status.as_u16(),
            path = %path,
            method = %method,
            duration_ms = duration_ms,
            "server error"
        );
    } else {
        tracing::info!(
            status = %status.as_u16(),
            path = %path,
            method = %method,
            duration_ms = duration_ms,
            "response sent"
        );
    }
}

/// Performance metrics collection
#[derive(Debug, Clone)]
pub struct RequestMetrics {
    pub path: String,
    pub method: String,
    pub status: u16,
    pub duration_ms: u128,
}

impl RequestMetrics {
    pub fn new(path: String, method: String, status: u16, duration_ms: u128) -> Self {
        Self {
            path,
            method,
            status,
            duration_ms,
        }
    }

    pub fn log(&self) {
        tracing::debug!(
            path = %self.path,
            method = %self.method,
            status = self.status,
            duration_ms = self.duration_ms,
            "request metrics"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::TestLogSubscriber;
    use tracing::Level;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::Registry;

    #[tokio::test]
    async fn test_request_logging_middleware() {
        let subscriber = TestLogSubscriber::new();
        let layer = subscriber.clone().into_layer::<Registry>();
        let registry = Registry::default().with(layer);

        let _guard = tracing::subscriber::set_default(registry);

        let route = warp::path!("test")
            .and(warp::get())
            .map(|| warp::reply::json(&"success"));

        let logged_route = with_request_logging(route);

        let response = warp::test::request()
            .method("GET")
            .path("/test")
            .reply(&logged_route)
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        let entries = subscriber.entries();
        assert!(entries
            .iter()
            .any(|e| e.level == Level::INFO && e.message.contains("incoming request")));
        assert!(entries
            .iter()
            .any(|e| e.level == Level::INFO && e.message.contains("request completed")));
    }

    #[test]
    fn test_log_response_success() {
        let subscriber = TestLogSubscriber::new();
        let layer = subscriber.clone().into_layer::<Registry>();
        let registry = Registry::default().with(layer);

        tracing::subscriber::with_default(registry, || {
            log_response(StatusCode::OK, "/api/test", "GET", 100);
        });

        let entries = subscriber.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, Level::INFO);
        assert!(entries[0].message.contains("response sent"));
        assert!(entries[0]
            .fields
            .iter()
            .any(|(k, v)| k == "status" && v.contains("200")));
    }

    #[test]
    fn test_log_response_client_error() {
        let subscriber = TestLogSubscriber::new();
        let layer = subscriber.clone().into_layer::<Registry>();
        let registry = Registry::default().with(layer);

        tracing::subscriber::with_default(registry, || {
            log_response(StatusCode::NOT_FOUND, "/api/missing", "GET", 50);
        });

        let entries = subscriber.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, Level::WARN);
        assert!(entries[0].message.contains("client error"));
        assert!(entries[0]
            .fields
            .iter()
            .any(|(k, v)| k == "status" && v.contains("404")));
    }

    #[test]
    fn test_log_response_server_error() {
        let subscriber = TestLogSubscriber::new();
        let layer = subscriber.clone().into_layer::<Registry>();
        let registry = Registry::default().with(layer);

        tracing::subscriber::with_default(registry, || {
            log_response(StatusCode::INTERNAL_SERVER_ERROR, "/api/error", "POST", 200);
        });

        let entries = subscriber.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, Level::ERROR);
        assert!(entries[0].message.contains("server error"));
        assert!(entries[0]
            .fields
            .iter()
            .any(|(k, v)| k == "status" && v.contains("500")));
    }

    #[test]
    fn test_request_metrics_creation() {
        let metrics = RequestMetrics::new("/api/test".to_string(), "GET".to_string(), 200, 150);

        assert_eq!(metrics.path, "/api/test");
        assert_eq!(metrics.method, "GET");
        assert_eq!(metrics.status, 200);
        assert_eq!(metrics.duration_ms, 150);
    }

    #[test]
    fn test_request_metrics_logging() {
        let subscriber = TestLogSubscriber::new();
        let layer = subscriber.clone().into_layer::<Registry>();
        let registry = Registry::default().with(layer);

        tracing::subscriber::with_default(registry, || {
            let metrics = RequestMetrics::new("/api/test".to_string(), "POST".to_string(), 201, 75);
            metrics.log();
        });

        let entries = subscriber.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, Level::DEBUG);
        assert!(entries[0].message.contains("request metrics"));
        assert!(entries[0]
            .fields
            .iter()
            .any(|(k, v)| k == "path" && v.contains("/api/test")));
        assert!(entries[0]
            .fields
            .iter()
            .any(|(k, v)| k == "method" && v.contains("POST")));
        assert!(entries[0]
            .fields
            .iter()
            .any(|(k, v)| k == "status" && v.contains("201")));
        assert!(entries[0]
            .fields
            .iter()
            .any(|(k, v)| k == "duration_ms" && v.contains("75")));
    }
}
