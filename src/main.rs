use axum::Router;
use std::net::SocketAddr;
use std::path::Path;
// use std::sync::mpsc::Receiver;
use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tower_http::services::ServeDir;
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::ClientConfig;
use twitch_irc::SecureTCPTransport;
use twitch_irc::TwitchIRCClient;

struct AppState {
    twitch_tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() {
    let (twitch_tx, _) = broadcast::channel::<String>(100);
    let mut set = JoinSet::new();
    set.spawn(twitch_bot(twitch_tx.clone()));
    set.spawn(web_page(twitch_tx.clone()));
    loop {}
}

async fn web_page(twitch_tx: broadcast::Sender<String>) {
    let app_state = Arc::new(AppState { twitch_tx });
    let addr = SocketAddr::from(([127, 0, 0, 1], 3314));
    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .nest_service("/", ServeDir::new(Path::new("html")))
        .with_state(app_state);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(stream: WebSocket, state: Arc<AppState>) {}

async fn twitch_bot(twitch_tx: broadcast::Sender<String>) {
    let config = ClientConfig::default();
    let (mut incoming_messages, client) =
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);
    let join_handle = tokio::spawn(async move {
        while let Some(message) = incoming_messages.recv().await {
            match message {
                twitch_irc::message::ServerMessage::Privmsg(payload) => {
                    println!("{}\n{}\n", payload.sender.name, payload.message_text);
                }
                _ => {
                    // dbg!(message);
                }
            }
        }
    });
    client.join("theidofalan".to_owned()).unwrap();
    join_handle.await.unwrap();
}
