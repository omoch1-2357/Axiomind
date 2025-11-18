use crate::session::{
    GameConfig, GameStateResponse, OpponentType, SessionError, SessionId, SessionManager,
};
use axiomind_engine::player::PlayerAction;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use warp::http::{self, StatusCode};
use warp::reply::{self, html, Response};
use warp::Reply;

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub seed: Option<u64>,
    pub level: Option<u8>,
    pub opponent_type: Option<OpponentType>,
}

impl CreateSessionRequest {
    fn into_config(self) -> GameConfig {
        let mut config = GameConfig::default();
        if let Some(seed) = self.seed {
            config.seed = Some(seed);
        }
        if let Some(level) = self.level {
            config.level = level;
        }
        if let Some(opponent_type) = self.opponent_type {
            config.opponent_type = opponent_type;
        }
        config
    }
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub session_id: SessionId,
    pub config: GameConfig,
    pub state: GameStateResponse,
}

#[derive(Debug, Deserialize)]
pub struct PlayerActionRequest {
    pub action: PlayerAction,
}

/// Creates a new game session with the specified configuration.
///
/// # HTTP Method and Path
/// - **Method**: POST
/// - **Path**: `/api/sessions`
///
/// # Purpose
/// Initializes a new poker game session with configurable parameters such as seed, blind level,
/// and opponent type (human or AI). Returns an HTML fragment for htmx-based rendering.
///
/// # Request Format
/// Expects JSON payload with optional fields:
/// ```json
/// {
///   "seed": 12345,           // Optional: RNG seed for reproducibility
///   "level": 3,              // Optional: Blind level (1-20)
///   "opponent_type": "ai:baseline"  // Optional: "human" or "ai:<strategy>"
/// }
/// ```
///
/// # Response Format
/// - **Success (201 Created)**: HTML fragment containing game state initialization script
/// - **Error (4xx/5xx)**: JSON error response with error code and message
///
/// # Error Cases
/// - `session_creation_failed`: Engine initialization fails
/// - `storage_poisoned`: Internal session storage lock is corrupted
///
/// # Arguments
/// * `sessions` - Shared reference to the session manager
/// * `request` - Deserialized request containing session configuration
///
/// # Returns
/// HTTP response with status 201 and HTML body on success, or error response on failure
pub async fn create_session(
    sessions: Arc<SessionManager>,
    request: CreateSessionRequest,
) -> Response {
    let config = request.into_config();

    match sessions.create_session(config.clone()) {
        Ok(session_id) => {
            // Return HTML for htmx to render the game state with 201 CREATED status
            let html_response = render_game_state(sessions, session_id).await;
            reply::with_status(html_response, StatusCode::CREATED).into_response()
        }
        Err(err) => session_error(err),
    }
}

/// Retrieves session information including configuration and current game state.
///
/// # HTTP Method and Path
/// - **Method**: GET
/// - **Path**: `/api/sessions/{session_id}`
///
/// # Purpose
/// Fetches complete session details including player states, board cards, pot size,
/// and available actions for the current player.
///
/// # Request Format
/// No request body. Session ID is provided as a URL path parameter.
///
/// # Response Format
/// - **Success (200 OK)**: JSON response with session data
/// ```json
/// {
///   "session_id": "uuid-string",
///   "config": { "seed": 42, "level": 1, "opponent_type": "ai:baseline" },
///   "state": { ... }
/// }
/// ```
/// - **Error (404 Not Found)**: Session does not exist
/// - **Error (410 Gone)**: Session has expired
///
/// # Error Cases
/// - `session_not_found`: No session with the given ID exists
/// - `session_expired`: Session exceeded inactivity timeout
///
/// # Arguments
/// * `sessions` - Shared reference to the session manager
/// * `session_id` - Unique identifier for the game session
///
/// # Returns
/// HTTP response with JSON body on success, or error response on failure
pub async fn get_session(sessions: Arc<SessionManager>, session_id: SessionId) -> Response {
    match assemble_session_response(&sessions, &session_id) {
        Ok(response) => success_response(StatusCode::OK, response),
        Err(err) => session_error(err),
    }
}

pub async fn get_session_state(sessions: Arc<SessionManager>, session_id: SessionId) -> Response {
    match sessions.state(&session_id) {
        Ok(state) => success_response(StatusCode::OK, state),
        Err(err) => session_error(err),
    }
}

/// Submits a player action for the current turn in an active game session.
///
/// # HTTP Method and Path
/// - **Method**: POST
/// - **Path**: `/api/sessions/{session_id}/actions`
///
/// # Purpose
/// Processes a player's action (Fold, Check, Call, Bet, Raise, AllIn) and advances
/// the game state. If the next player is AI-controlled, their action is automatically
/// processed and broadcast via the event bus.
///
/// # Request Format
/// Expects JSON payload with the player action:
/// ```json
/// {
///   "action": "Check"
/// }
/// ```
/// or with an amount for Bet/Raise:
/// ```json
/// {
///   "action": { "Bet": 200 }
/// }
/// ```
///
/// # Response Format
/// - **Success (202 Accepted)**: JSON event describing the processed action
/// ```json
/// {
///   "session_id": "uuid",
///   "player_id": 0,
///   "action": "Check"
/// }
/// ```
/// - **Error (400 Bad Request)**: Invalid action for current game state
/// - **Error (404 Not Found)**: Session does not exist
///
/// # Error Cases
/// - `invalid_action`: Action is not allowed in the current state
/// - `session_not_found`: Session ID does not exist
/// - `session_expired`: Session has timed out
///
/// # Arguments
/// * `sessions` - Shared reference to the session manager
/// * `session_id` - Unique identifier for the game session
/// * `request` - Deserialized action request
///
/// # Returns
/// HTTP response with status 202 and event JSON on success, or error response on failure
pub async fn submit_action(
    sessions: Arc<SessionManager>,
    session_id: SessionId,
    request: PlayerActionRequest,
) -> Response {
    match sessions.process_action(&session_id, request.action) {
        Ok(event) => success_response(StatusCode::ACCEPTED, event),
        Err(err) => session_error(err),
    }
}

/// Deletes an existing session and broadcasts a game-ended event.
///
/// # HTTP Method and Path
/// - **Method**: DELETE
/// - **Path**: `/api/sessions/{session_id}`
///
/// # Purpose
/// Terminates an active game session, removes it from the session manager's storage,
/// and notifies all subscribers via the event bus that the game has ended.
///
/// # Request Format
/// No request body. Session ID is provided as a URL path parameter.
///
/// # Response Format
/// - **Success (204 No Content)**: Empty response body
/// - **Error (404 Not Found)**: Session does not exist
///
/// # Error Cases
/// - `session_not_found`: No session with the given ID exists
///
/// # Arguments
/// * `sessions` - Shared reference to the session manager
/// * `session_id` - Unique identifier for the game session to delete
///
/// # Returns
/// HTTP response with status 204 on success, or error response on failure
pub async fn delete_session(sessions: Arc<SessionManager>, session_id: SessionId) -> Response {
    match sessions.delete_session(&session_id) {
        Ok(()) => empty_response(StatusCode::NO_CONTENT),
        Err(err) => session_error(err),
    }
}

fn assemble_session_response(
    sessions: &SessionManager,
    session_id: &SessionId,
) -> Result<SessionResponse, SessionError> {
    let config = sessions.config(session_id)?;
    let state = sessions.state(session_id)?;
    Ok(SessionResponse {
        session_id: session_id.clone(),
        config,
        state,
    })
}

fn success_response<T>(status: StatusCode, body: T) -> Response
where
    T: Serialize,
{
    reply::with_status(reply::json(&body), status).into_response()
}

fn empty_response(status: StatusCode) -> Response {
    http::Response::builder()
        .status(status)
        .body(warp::hyper::Body::empty())
        .expect("build empty response")
}

fn session_error(err: SessionError) -> Response {
    use crate::errors::IntoErrorResponse;
    err.into_http_response()
}

pub async fn lobby(_sessions: Arc<SessionManager>) -> Response {
    let mut level_options = String::new();
    for lvl in 1u8..=20 {
        level_options.push_str(&format!(r#"<option value="{0}">Level {0}</option>"#, lvl));
    }
    let html_content = format!(
        r##"
        <div class="lobby-container">
            <h2>Game Lobby</h2>
            <p class="lobby-message">Click "Start Game" to begin a new poker session</p>
            <form
            hx-post="/api/sessions"
            hx-target="#table"
            hx-swap="innerHTML"
            hx-ext="json-enc"
            hx-vals='js:{{level: parseInt(document.getElementById("level").value), opponent_type: document.getElementById("opponent_type").value}}'
            class="start-game-form"
        >
            <div class="form-group">
                <label for="level">Blind Level:</label>
                <select name="level" id="level">
                    {level_options}
                </select>
            </div>
            <div class="form-group">
                <label for="opponent_type">Opponent:</label>
                <select name="opponent_type" id="opponent_type">
                    <option value="ai:baseline">AI (Baseline)</option>
                    <option value="ai:aggressive">AI (Aggressive)</option>
                    <option value="human">Human</option>
                </select>
            </div>
            <button type="submit" class="start-game-btn">Start Game</button>
        </form>
    </div>
    "##,
        level_options = level_options
    );
    html(html_content).into_response()
}

pub async fn render_game_state(sessions: Arc<SessionManager>, session_id: SessionId) -> Response {
    match sessions.state(&session_id) {
        Ok(state) => {
            let state_json = match serde_json::to_string(&state) {
                Ok(json) => json,
                Err(err) => {
                    tracing::error!("Failed to serialize game state: {}", err);
                    return session_error(SessionError::EngineError("serialization failed".into()));
                }
            };
            let html_content = format!(
                r##"
                <div id="game-container" data-session-id="{}">
                    <script>
                        (function() {{
                            const state = {};
                            const tableHtml = renderPokerTable(state);
                            const controlsHtml = renderBettingControls(state);

                            // Use setTimeout to ensure DOM is ready after htmx swap
                            setTimeout(function() {{
                                const tableEl = document.getElementById('table');
                                const controlsEl = document.getElementById('controls');
                                if (tableEl) tableEl.innerHTML = tableHtml;
                                if (controlsEl) controlsEl.innerHTML = controlsHtml;
                            }}, 0);

                            // Setup SSE connection
                            if (!window.eventSource) {{
                                window.eventSource = setupEventStream('{}');
                            }}
                        }})();
                    </script>
                    <div id="table-content"></div>
                </div>
                "##,
                session_id, state_json, session_id
            );
            html(html_content).into_response()
        }
        Err(err) => session_error(err),
    }
}
