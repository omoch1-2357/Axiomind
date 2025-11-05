use serde::Serialize;
use warp::reply::Json;

#[derive(Serialize)]
struct HealthBody {
    status: &'static str,
}

pub fn health() -> Json {
    warp::reply::json(&HealthBody { status: "ok" })
}
