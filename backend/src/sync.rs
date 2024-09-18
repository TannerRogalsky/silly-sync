use serde::{Deserialize, Serialize};
use worker::*;

const GAME_STATE_KEY: &str = "GAME_STATE";

#[derive(Default, Serialize, Deserialize)]
struct WebsocketAttachment {
    id: String,
    name: Option<String>,
}

struct Session {
    ws: WebSocket,
    #[allow(unused)]
    meta: WebsocketAttachment,
}

#[derive(Default, Serialize, Deserialize)]
struct User {
    x: f32,
    y: f32,
    avatar: String,
}

#[derive(Default, Serialize, Deserialize)]
struct GameState {
    users: std::collections::HashMap<String, User>,
}

#[durable_object]
pub struct SillySync {
    state: State,
    sessions: std::collections::HashMap<String, Session>,
}

impl SillySync {
    fn broadcast(&self, payload: &GameState) -> Result<()> {
        let payload = serde_json::to_string(payload)?;
        for (_id, session) in self.sessions.iter() {
            // todo: handle error by disconnecting the ws and removing the session
            session.ws.send_with_str(&payload)?;
        }
        Ok(())
    }

    async fn handle_close_or_error(&mut self, ws: WebSocket) -> Result<()> {
        if let Some(attachment) = ws.deserialize_attachment::<WebsocketAttachment>()? {
            self.sessions.remove(&attachment.id);

            if self.sessions.is_empty() {
                // reset the durable object
                self.state.storage().delete_all().await?;
            } else {
                let mut state = self
                    .state
                    .storage()
                    .get::<GameState>(GAME_STATE_KEY)
                    .await?;
                state.users.remove(&attachment.id);
                self.state.storage().put(GAME_STATE_KEY, &state).await?;
                self.broadcast(&state)?;
            }
        }
        Ok(())
    }
}

#[durable_object]
impl DurableObject for SillySync {
    fn new(state: State, _env: Env) -> Self {
        let sessions = state
            .get_websockets()
            .into_iter()
            .filter_map(|ws| {
                let meta = ws
                    .deserialize_attachment::<WebsocketAttachment>()
                    .ok()
                    .flatten()?;
                Some((meta.id.clone(), Session { meta, ws }))
            })
            .collect();
        Self { state, sessions }
    }

    async fn fetch(&mut self, req: Request) -> Result<Response> {
        let headers = req.headers();
        let is_upgrade = headers
            .get("Upgrade")?
            .as_ref()
            .is_some_and(|header| header == "websocket");
        if is_upgrade {
            let _ip = headers.get("CF-Connecting-IP")?;
            let WebSocketPair { client, server } = WebSocketPair::new()?;

            self.state.accept_web_socket(&server);
            let url = req.url()?;
            let (_, user_id) = url
                .query_pairs()
                .find(|(key, _value)| key == "user_id")
                .ok_or_else(|| Error::RustError(String::from("No user_id param")))?;
            let meta = WebsocketAttachment {
                id: user_id.to_string(),
                name: None,
            };
            server.serialize_attachment(&meta)?;
            self.sessions
                .insert(user_id.to_string(), Session { meta, ws: server });

            Response::from_websocket(client)
        } else {
            let state = self
                .state
                .storage()
                .get::<GameState>(GAME_STATE_KEY)
                .await
                .unwrap_or_default();
            Response::from_json(&state)
        }
    }

    async fn websocket_message(
        &mut self,
        _ws: WebSocket,
        message: WebSocketIncomingMessage,
    ) -> Result<()> {
        let state: GameState = match message {
            WebSocketIncomingMessage::String(data) => serde_json::from_str(&data)?,
            WebSocketIncomingMessage::Binary(data) => serde_json::from_slice(&data)?,
        };

        self.state.storage().put(GAME_STATE_KEY, &state).await?;
        self.broadcast(&state)?;

        Ok(())
    }

    async fn websocket_close(
        &mut self,
        ws: WebSocket,
        _code: usize,
        _reason: String,
        _was_clean: bool,
    ) -> Result<()> {
        self.handle_close_or_error(ws).await
    }

    async fn websocket_error(&mut self, ws: WebSocket, _error: Error) -> Result<()> {
        self.handle_close_or_error(ws).await
    }
}
