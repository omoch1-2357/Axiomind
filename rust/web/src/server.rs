use crate::events::EventBus;
use crate::history::{HandFilter, HistoryStore};
use crate::session::{SessionError, SessionManager};
use crate::static_handler::StaticHandler;
use std::convert::Infallible;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

use crate::handlers;
use std::net::SocketAddr;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::info;
use warp::filters::BoxedFilter;
use warp::http::StatusCode;
use warp::reply::Reply;
use warp::{reject, reply, Filter, Rejection};

use std::net::ToSocketAddrs;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    host: String,
    port: u16,
    static_dir: PathBuf,
}

impl ServerConfig {
    pub fn new(host: impl Into<String>, port: u16, static_dir: impl Into<PathBuf>) -> Self {
        Self {
            host: host.into(),
            port,
            static_dir: static_dir.into(),
        }
    }

    pub fn for_tests() -> Self {
        let dir = std::env::temp_dir().join("axm_web_static");
        Self::new("127.0.0.1", 0, dir)
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn static_dir(&self) -> &Path {
        &self.static_dir
    }
}

#[derive(Debug, Clone)]
pub struct AppContext {
    config: ServerConfig,
    event_bus: Arc<EventBus>,
    sessions: Arc<SessionManager>,
    static_handler: Arc<StaticHandler>,
    history: Arc<HistoryStore>,
}

impl AppContext {
    pub fn new(config: ServerConfig) -> Result<Self, ServerError> {
        if !config.static_dir().exists() {
            fs::create_dir_all(config.static_dir())
                .map_err(|err| ServerError::ConfigError(err.to_string()))?;
        }

        let event_bus = Arc::new(EventBus::new());
        let history = Arc::new(HistoryStore::new());
        let sessions = Arc::new(SessionManager::with_history(
            Arc::clone(&event_bus),
            Arc::clone(&history),
        ));
        let static_handler = Arc::new(StaticHandler::new(config.static_dir().to_path_buf()));

        Ok(Self::new_with_dependencies(
            config,
            event_bus,
            sessions,
            static_handler,
            history,
        ))
    }

    pub fn new_with_dependencies(
        config: ServerConfig,
        event_bus: Arc<EventBus>,
        sessions: Arc<SessionManager>,
        static_handler: Arc<StaticHandler>,
        history: Arc<HistoryStore>,
    ) -> Self {
        Self {
            config,
            event_bus,
            sessions,
            static_handler,
            history,
        }
    }

    pub fn new_for_tests() -> Self {
        Self::new(ServerConfig::for_tests()).expect("test context")
    }

    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    pub fn event_bus(&self) -> Arc<EventBus> {
        Arc::clone(&self.event_bus)
    }

    pub fn sessions(&self) -> Arc<SessionManager> {
        Arc::clone(&self.sessions)
    }

    pub fn static_handler(&self) -> Arc<StaticHandler> {
        Arc::clone(&self.static_handler)
    }

    pub fn history(&self) -> Arc<HistoryStore> {
        Arc::clone(&self.history)
    }
}

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Failed to bind to address: {0}")]
    BindError(#[from] std::io::Error),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Session error: {0}")]
    SessionError(#[from] SessionError),
}

#[derive(Debug, Clone)]
pub struct WebServer {
    context: AppContext,
}

impl WebServer {
    pub fn new(config: ServerConfig) -> Result<Self, ServerError> {
        let context = AppContext::new(config)?;
        Ok(Self { context })
    }

    pub fn from_context(context: AppContext) -> Self {
        Self { context }
    }

    pub fn context(&self) -> &AppContext {
        &self.context
    }

    pub async fn start(self) -> Result<ServerHandle, ServerError> {
        let WebServer { context } = self;
        let config = context.config().clone();
        let bind_addr = Self::bind_addr(&config)?;

        let preflight = if bind_addr.port() != 0 {
            Some(std::net::TcpListener::bind(bind_addr).map_err(ServerError::BindError)?)
        } else {
            None
        };
        drop(preflight);

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let routes = Self::routes(&context);
        let shutdown_signal = async move {
            let _ = shutdown_rx.await;
        };

        let (addr, server_future) = warp::serve(routes)
            .try_bind_with_graceful_shutdown(bind_addr, shutdown_signal)
            .map_err(Self::map_warp_error)?;

        info(format!("web server listening on http://{}", addr));

        let task = tokio::spawn(async move {
            server_future.await;
            Ok(())
        });

        Ok(ServerHandle::new(addr, shutdown_tx, task, context))
    }

    fn bind_addr(config: &ServerConfig) -> Result<SocketAddr, ServerError> {
        let host = config.host();

        if let Ok(addr) = host.parse::<SocketAddr>() {
            return Ok(addr);
        }

        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            return Ok(SocketAddr::new(ip, config.port()));
        }

        let candidate = format!("{}:{}", host, config.port());
        let mut addrs = candidate.to_socket_addrs().map_err(|err| {
            ServerError::ConfigError(format!("failed to resolve address `{candidate}`: {err}"))
        })?;

        addrs.next().ok_or_else(|| {
            ServerError::ConfigError(format!("failed to resolve address `{candidate}`"))
        })
    }

    fn map_warp_error(err: warp::Error) -> ServerError {
        use std::error::Error as StdError;

        if let Some(source) = err.source() {
            if let Some(io_err) = source.downcast_ref::<std::io::Error>() {
                let recreated = std::io::Error::new(io_err.kind(), io_err.to_string());
                return ServerError::BindError(recreated);
            }
        }

        ServerError::ConfigError(err.to_string())
    }

    fn routes(context: &AppContext) -> BoxedFilter<(warp::reply::Response,)> {
        let health = Self::health_route();
        let static_routes = Self::static_routes(context);
        let api_routes = Self::api_routes(context);
        let history_routes = Self::history_routes(context);
        let sse_routes = Self::sse_routes(context);

        health
            .or(static_routes)
            .unify()
            .or(api_routes)
            .unify()
            .or(history_routes)
            .unify()
            .or(sse_routes)
            .unify()
            .boxed()
    }

    fn health_route() -> BoxedFilter<(warp::reply::Response,)> {
        warp::path("health")
            .and(warp::get())
            .and(warp::path::end())
            .map(|| handlers::health::health().into_response())
            .boxed()
    }

    fn static_routes(context: &AppContext) -> BoxedFilter<(warp::reply::Response,)> {
        let handler = context.static_handler();

        let index = warp::path::end()
            .and(warp::get())
            .and(Self::with_static_handler(handler.clone()))
            .and_then(|handler: Arc<StaticHandler>| async move {
                let response = handler
                    .index()
                    .await
                    .unwrap_or_else(|err| handler.error_response(err));
                Ok::<_, Infallible>(response)
            });

        let assets = warp::path("static")
            .and(warp::path::tail())
            .and(warp::get())
            .and(Self::with_static_handler(handler))
            .and_then(
                |tail: warp::path::Tail, handler: Arc<StaticHandler>| async move {
                    let response = handler
                        .asset(tail.as_str())
                        .await
                        .unwrap_or_else(|err| handler.error_response(err));
                    Ok::<_, Infallible>(response)
                },
            );

        index.or(assets).unify().boxed()
    }

    fn api_routes(context: &AppContext) -> BoxedFilter<(warp::reply::Response,)> {
        let sessions = context.sessions();

        let lobby = warp::path!("api" / "game" / "lobby")
            .and(warp::get())
            .and(Self::with_session_manager(sessions.clone()))
            .and_then(|sessions: Arc<SessionManager>| async move {
                let response = handlers::lobby(sessions).await;
                Ok::<_, Infallible>(response)
            });

        let create = warp::path!("api" / "sessions")
            .and(warp::post())
            .and(Self::with_session_manager(sessions.clone()))
            .and(warp::body::json())
            .and_then(
                |sessions: Arc<SessionManager>,
                 request: handlers::CreateSessionRequest| async move {
                    let response = handlers::create_session(sessions, request).await;
                    Ok::<_, Infallible>(response)
                },
            );

        let info = warp::path!("api" / "sessions" / String)
            .and(warp::get())
            .and(Self::with_session_manager(sessions.clone()))
            .and_then(
                |session_id: String, sessions: Arc<SessionManager>| async move {
                    let response = handlers::get_session(sessions, session_id).await;
                    Ok::<_, Infallible>(response)
                },
            );

        let state = warp::path!("api" / "sessions" / String / "state")
            .and(warp::get())
            .and(Self::with_session_manager(sessions.clone()))
            .and_then(
                |session_id: String, sessions: Arc<SessionManager>| async move {
                    let response = handlers::get_session_state(sessions, session_id).await;
                    Ok::<_, Infallible>(response)
                },
            );

        let actions = warp::path!("api" / "sessions" / String / "actions")
            .and(warp::post())
            .and(Self::with_session_manager(sessions.clone()))
            .and(warp::body::json())
            .and_then(
                |session_id: String,
                 sessions: Arc<SessionManager>,
                 request: handlers::PlayerActionRequest| async move {
                    let response = handlers::submit_action(sessions, session_id, request).await;
                    Ok::<_, Infallible>(response)
                },
            );

        let delete = warp::path!("api" / "sessions" / String)
            .and(warp::delete())
            .and(Self::with_session_manager(sessions))
            .and_then(
                |session_id: String, sessions: Arc<SessionManager>| async move {
                    let response = handlers::delete_session(sessions, session_id).await;
                    Ok::<_, Infallible>(response)
                },
            );

        lobby
            .or(create)
            .unify()
            .or(state)
            .unify()
            .or(actions)
            .unify()
            .or(info)
            .unify()
            .or(delete)
            .unify()
            .boxed()
    }

    fn sse_routes(context: &AppContext) -> BoxedFilter<(warp::reply::Response,)> {
        let sessions = context.sessions();
        let event_bus = context.event_bus();

        warp::path!("api" / "sessions" / String / "events")
            .and(warp::get())
            .and(Self::with_session_manager(sessions))
            .and(Self::with_event_bus(event_bus))
            .and_then(
                |session_id: String,
                 sessions: Arc<SessionManager>,
                 event_bus: Arc<EventBus>| async move {
                    let response =
                        handlers::sse::stream_events(session_id, sessions, event_bus).await;
                    Ok::<_, Infallible>(response)
                },
            )
            .boxed()
    }

    fn history_routes(context: &AppContext) -> BoxedFilter<(warp::reply::Response,)> {
        let history = context.history();

        let recent = warp::path!("api" / "history")
            .and(warp::get())
            .and(warp::query::<handlers::history::GetHistoryQuery>())
            .and(Self::with_history_store(history.clone()))
            .and_then(
                |query: handlers::history::GetHistoryQuery, history: Arc<HistoryStore>| async move {
                    let hands = history
                        .get_recent_hands(query.limit)
                        .map_err(|_| reject::not_found())?;
                    Ok::<_, Rejection>(reply::json(&hands))
                },
            )
            .map(|response: warp::reply::Json| response.into_response());

        let by_id = warp::path!("api" / "history" / String)
            .and(warp::get())
            .and(Self::with_history_store(history.clone()))
            .and_then(|hand_id: String, history: Arc<HistoryStore>| async move {
                let hand = history
                    .get_hand(&hand_id)
                    .map_err(|_| reject::not_found())?;
                match hand {
                    Some(h) => {
                        Ok::<_, Rejection>(reply::with_status(reply::json(&h), StatusCode::OK))
                    }
                    None => Err(reject::not_found()),
                }
            })
            .map(|response: warp::reply::WithStatus<warp::reply::Json>| response.into_response());

        let filter = warp::path!("api" / "history" / "filter")
            .and(warp::post())
            .and(warp::body::json())
            .and(Self::with_history_store(history.clone()))
            .and_then(
                |filter: HandFilter, history: Arc<HistoryStore>| async move {
                    let hands = history
                        .filter_hands(filter)
                        .map_err(|_| reject::not_found())?;
                    Ok::<_, Rejection>(reply::json(&hands))
                },
            )
            .map(|response: warp::reply::Json| response.into_response());

        let stats = warp::path!("api" / "history" / "stats")
            .and(warp::get())
            .and(Self::with_history_store(history))
            .and_then(|history: Arc<HistoryStore>| async move {
                let stats = history.calculate_stats().map_err(|_| reject::not_found())?;
                Ok::<_, Rejection>(reply::json(&stats))
            })
            .map(|response: warp::reply::Json| response.into_response());

        recent
            .or(filter)
            .unify()
            .or(stats)
            .unify()
            .or(by_id)
            .unify()
            .boxed()
    }

    fn with_static_handler(
        handler: Arc<StaticHandler>,
    ) -> impl Filter<Extract = (Arc<StaticHandler>,), Error = Infallible> + Clone {
        warp::any().map(move || handler.clone())
    }

    fn with_session_manager(
        sessions: Arc<SessionManager>,
    ) -> impl Filter<Extract = (Arc<SessionManager>,), Error = Infallible> + Clone {
        warp::any().map(move || Arc::clone(&sessions))
    }

    fn with_event_bus(
        event_bus: Arc<EventBus>,
    ) -> impl Filter<Extract = (Arc<EventBus>,), Error = Infallible> + Clone {
        warp::any().map(move || Arc::clone(&event_bus))
    }

    fn with_history_store(
        history: Arc<HistoryStore>,
    ) -> impl Filter<Extract = (Arc<HistoryStore>,), Error = Infallible> + Clone {
        warp::any().map(move || Arc::clone(&history))
    }
}

#[derive(Debug)]
pub struct ServerHandle {
    addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<Result<(), ServerError>>>,
    context: AppContext,
}

impl ServerHandle {
    fn new(
        addr: SocketAddr,
        shutdown: oneshot::Sender<()>,
        task: JoinHandle<Result<(), ServerError>>,
        context: AppContext,
    ) -> Self {
        Self {
            addr,
            shutdown: Some(shutdown),
            task: Some(task),
            context,
        }
    }

    pub fn address(&self) -> SocketAddr {
        self.addr
    }

    pub fn context(&self) -> &AppContext {
        &self.context
    }

    pub async fn shutdown(mut self) -> Result<(), ServerError> {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }

        if let Some(task) = self.task.take() {
            match task.await {
                Ok(result) => result?,
                Err(err) => {
                    return Err(ServerError::ConfigError(format!(
                        "server task join error: {err}"
                    )))
                }
            }
        }

        Ok(())
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }

        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

mod tracing {
    pub fn info(message: impl AsRef<str>) {
        println!("{}", message.as_ref());
    }
}
