mod sync;
// mod error;
// mod oauth;
// mod session_store;
// mod user;
// mod ws;

pub use sync::SillySync;
use worker::*;

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    let router = Router::new()
        .get_async("/", index)
        .post_async("/token", token)
        .get_async("/room/:room_name", |req, ctx| async move {
            let name = match ctx.param("room_name") {
                Some(name) => name,
                None => return Response::error("Expected a room name.", 404),
            };
            let namespace = ctx.env.durable_object("SILLY_SYNC")?;
            let id = namespace.id_from_name(name)?;
            let room_object = id.get_stub()?;
            room_object.fetch_with_request(req).await
        });

    router.run(req, env).await
}

async fn index(_request: Request, _ctx: RouteContext<()>) -> Result<Response> {
    Response::from_html("HI")
}

#[derive(serde::Deserialize)]
struct TokenRequest {
    code: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct TokenResponse {
    access_token: String,
}

async fn token(mut request: Request, ctx: RouteContext<()>) -> Result<Response> {
    let payload = request.json::<TokenRequest>().await?;

    let client_id = ctx
        .env
        .secret("CLIENT_ID")
        .map_err(|_| Error::RustError("Missing CLIENT_ID!".to_string()))?
        .to_string();
    let client_secret = ctx
        .env
        .secret("CLIENT_SECRET")
        .map_err(|_| Error::RustError("Missing CLIENT_SECRET!".to_string()))?
        .to_string();

    #[derive(serde::Serialize)]
    struct Body {
        client_id: String,
        client_secret: String,
        grant_type: &'static str,
        code: String,
    }

    let body = Body {
        client_id,
        client_secret,
        grant_type: "authorization_code",
        code: payload.code,
    };

    let client = reqwest::Client::new();
    let response = client
        .post("https://discord.com/api/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(serde_urlencoded::to_string(&body).unwrap())
        .send()
        .await
        .map_err(|err| Error::RustError(format!("{}", err)))?
        .json::<TokenResponse>()
        .await
        .map_err(|err| Error::RustError(format!("{}", err)))?;

    Response::from_json(&response)
}
