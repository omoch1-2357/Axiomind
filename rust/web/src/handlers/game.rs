use crate::session::{
    GameConfig, GameStateResponse, OpponentType, SessionError, SessionId, SessionManager,
};
use axm_engine::player::PlayerAction;
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

pub async fn create_session(
    sessions: Arc<SessionManager>,
    request: CreateSessionRequest,
) -> Response {
    let config = request.into_config();

    match sessions.create_session(config.clone()) {
        Ok(session_id) => match sessions.state(&session_id) {
            Ok(state) => success_response(
                StatusCode::CREATED,
                SessionResponse {
                    session_id,
                    config,
                    state,
                },
            ),
            Err(err) => {
                let _ = sessions.delete_session(&session_id);
                session_error(err)
            }
        },
        Err(err) => session_error(err),
    }
}

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
    let html_content = r##"
    <div class="lobby-container">
        <h2>Game Lobby</h2>
        <p class="lobby-message">Click "Start Game" to begin a new poker session</p>
        <form
            hx-post="/api/sessions"
            hx-target="#table"
            hx-swap="innerHTML"
            hx-ext="json-enc"
            class="start-game-form"
        >
            <div class="form-group">
                <label for="level">Blind Level:</label>
                <select name="level" id="level">
                    <option value="1">Level 1 (50/100)</option>
                    <option value="2">Level 2 (100/200)</option>
                    <option value="3">Level 3 (200/400)</option>
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
    "##;
    html(html_content).into_response()
}

pub async fn render_game_state(sessions: Arc<SessionManager>, session_id: SessionId) -> Response {
    match sessions.state(&session_id) {
        Ok(state) => {
            let state_json = serde_json::to_string(&state).unwrap_or_default();
            let html_content = format!(
                r##"
                <div id="game-container" data-session-id="{}">
                    <script>
                        (function() {{
                            const state = {};
                            const tableHtml = renderPokerTable(state);
                            const controlsHtml = renderBettingControls(state);
                            document.getElementById('table').innerHTML = tableHtml;
                            document.getElementById('controls').innerHTML = controlsHtml;

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
