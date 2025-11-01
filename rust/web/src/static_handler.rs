use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use mime_guess::{mime, MimeGuess};
use tokio::fs;
use warp::http::{header::HeaderValue, Response, StatusCode};
use warp::hyper::Body;

#[derive(Debug, thiserror::Error)]
pub enum StaticError {
    #[error("asset not found")]
    NotFound,
    #[error("asset io error: {0}")]
    Io(#[from] std::io::Error),
}

impl crate::errors::IntoErrorResponse for StaticError {
    fn status_code(&self) -> warp::http::StatusCode {
        use warp::http::StatusCode;
        match self {
            StaticError::NotFound => StatusCode::NOT_FOUND,
            StaticError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            StaticError::NotFound => "static_not_found",
            StaticError::Io(_) => "static_io_error",
        }
    }

    fn error_message(&self) -> String {
        self.to_string()
    }

    fn severity(&self) -> crate::errors::ErrorSeverity {
        use crate::errors::ErrorSeverity;
        match self {
            StaticError::NotFound => ErrorSeverity::Client,
            StaticError::Io(_) => ErrorSeverity::Server,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StaticHandler {
    root: Arc<PathBuf>,
    cache_header: HeaderValue,
}

impl StaticHandler {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            root: Arc::new(root),
            cache_header: HeaderValue::from_static("public, max-age=86400"),
        }
    }

    pub fn root(&self) -> &Path {
        self.root.as_path()
    }

    pub async fn index(&self) -> Result<warp::reply::Response, StaticError> {
        self.serve_relative("index.html").await
    }

    pub async fn asset(&self, path: &str) -> Result<warp::reply::Response, StaticError> {
        if path.is_empty() {
            return Err(StaticError::NotFound);
        }
        self.serve_relative(path).await
    }

    pub fn error_response(&self, error: StaticError) -> warp::reply::Response {
        match error {
            StaticError::NotFound => self.not_found_response(),
            StaticError::Io(_) => self.internal_error_response(),
        }
    }

    fn not_found_response(&self) -> warp::reply::Response {
        let mut response = Response::new(Body::from("Not Found"));
        *response.status_mut() = StatusCode::NOT_FOUND;
        response.headers_mut().insert(
            warp::http::header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        );
        response
    }

    fn internal_error_response(&self) -> warp::reply::Response {
        let mut response = Response::new(Body::from("Internal Server Error"));
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        response.headers_mut().insert(
            warp::http::header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        );
        response
    }

    async fn serve_relative(&self, relative: &str) -> Result<warp::reply::Response, StaticError> {
        let resolved = self.resolve(relative)?;
        let bytes = match fs::read(&resolved).await {
            Ok(data) => data,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(StaticError::NotFound)
            }
            Err(err) => return Err(StaticError::Io(err)),
        };

        let mime = MimeGuess::from_path(&resolved).first_or_octet_stream();
        Ok(self.build_response(bytes, mime))
    }

    fn build_response(&self, bytes: Vec<u8>, mime: mime::Mime) -> warp::reply::Response {
        let mut response = Response::new(Body::from(bytes));
        let mut content_type = mime.essence_str().to_string();
        if mime.type_() == mime::TEXT {
            content_type.push_str("; charset=utf-8");
        }

        response.headers_mut().insert(
            warp::http::header::CONTENT_TYPE,
            HeaderValue::from_str(&content_type)
                .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
        );
        response
            .headers_mut()
            .insert(warp::http::header::CACHE_CONTROL, self.cache_header.clone());
        response
    }

    fn resolve(&self, path: &str) -> Result<PathBuf, StaticError> {
        let mut buf = PathBuf::new();
        for comp in Path::new(path).components() {
            match comp {
                Component::Normal(seg) => buf.push(seg),
                Component::CurDir => {}
                Component::RootDir => {}
                Component::Prefix(_) | Component::ParentDir => return Err(StaticError::NotFound),
            }
        }

        if buf.as_os_str().is_empty() {
            return Err(StaticError::NotFound);
        }

        Ok(self.root.join(buf))
    }
}
