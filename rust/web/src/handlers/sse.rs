use crate::events::{EventBus, EventSubscription, GameEvent};
use crate::session::{SessionError, SessionId, SessionManager};
use serde::Serialize;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;
use warp::http::{self, StatusCode};
use warp::reply::{self, Response};
use warp::sse;
use warp::Reply;

pub async fn stream_events(
    session_id: SessionId,
    sessions: Arc<SessionManager>,
    event_bus: Arc<EventBus>,
) -> Response {
    match sessions.get_session(&session_id) {
        Ok(_) => {}
        Err(SessionError::NotFound(_)) | Err(SessionError::Expired(_)) => {
            return error_response(
                StatusCode::NOT_FOUND,
                "session_not_found",
                format!("session `{session_id}` was not found"),
            );
        }
        Err(err) => {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "session_error",
                err.to_string(),
            );
        }
    }

    let subscription = event_bus.subscribe(session_id);
    let stream = subscription_stream(subscription);
    let keep_alive = sse::keep_alive()
        .interval(Duration::from_secs(15))
        .text(":keep-alive\n");

    let reply = sse::reply(keep_alive.stream(stream));
    reply::with_header(reply, http::header::CACHE_CONTROL, "no-cache").into_response()
}

fn subscription_stream(
    subscription: EventSubscription,
) -> impl tokio_stream::Stream<Item = Result<sse::Event, Infallible>> {
    let mut subscription = subscription;
    let (_, placeholder_rx) = mpsc::unbounded_channel();
    let receiver = std::mem::replace(&mut subscription.receiver, placeholder_rx);
    let subscription = Arc::new(subscription);

    UnboundedReceiverStream::new(receiver).map(move |event| {
        let _keep_alive = Arc::clone(&subscription);
        Ok(render_event(event))
    })
}

fn render_event(event: GameEvent) -> sse::Event {
    match serde_json::to_string(&event) {
        Ok(json) => sse::Event::default().event("game_event").data(json),
        Err(err) => {
            let fallback = serde_json::json!({
                "type": "error",
                "message": format!("failed to serialize game event: {err}")
            })
            .to_string();
            sse::Event::default().event("game_event").data(fallback)
        }
    }
}

fn error_response(status: StatusCode, error: &'static str, message: String) -> Response {
    #[derive(Serialize)]
    struct ErrorBody<'a> {
        error: &'a str,
        message: String,
    }

    let body = ErrorBody { error, message };
    reply::with_status(reply::json(&body), status).into_response()
}
