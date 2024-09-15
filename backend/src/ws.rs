use axum::{extract::WebSocketUpgrade, response::IntoResponse};

pub async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    // ws.on_upgrade(callback)
    unimplemented!()
}
