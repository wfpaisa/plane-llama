// Servidor de prueba sin modelo ni GPU, compatible con la API de llama-server.
use axum::{
    Json, Router,
    routing::{get, post},
};
use serde_json::{Value, json};
use std::time::Duration;
fn setting(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.iter().any(|a| a == "--list-devices") {
        println!("Available devices:\n  CUDA0: NVIDIA Fake GPU (16384 MiB, 15000 MiB free)");
        return;
    }
    let value = |flag: &str| args.windows(2).find(|a| a[0] == flag).map(|a| a[1].clone());
    let host = value("--host").unwrap_or("127.0.0.1".into());
    let port = value("--port").unwrap_or("8080".into());
    let mut shutdown = Box::pin(async {
        let mut signal =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
        signal.recv().await;
    });

    println!("loading model");
    if std::env::var("FAKE_CRASH").as_deref() == Ok("1") {
        std::process::exit(1);
    }
    let load = setting("FAKE_LOAD_MS", 300);
    tokio::select! { _ = &mut shutdown => return, _ = tokio::time::sleep(Duration::from_millis(load)) => {} }
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
        .await
        .unwrap();
    println!("model loaded in {load} ms\nserver is listening");
    let app = Router::new().route("/health", get(|| async { Json(json!({"status":"ok"})) }))
        .route("/v1/chat/completions", post(|Json(body): Json<Value>| async move {
            println!("fake request: {body}");
            tokio::time::sleep(Duration::from_millis(setting("FAKE_CHAT_MS", 50))).await;
            tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(setting("FAKE_TIMING_DELAY_MS", 0))).await;
            println!("prompt eval time = 100.00 ms / 20 tokens (200.00 tokens per second)\neval time = 200.00 ms / 10 tokens (50.00 tokens per second)\ndraft acceptance = 0.75 (3 accepted / 4 generated)");
            });
            Json(json!({"id":"fake", "object":"chat.completion", "choices":[{"message":{"role":"assistant","content":"Quedan 9 ovejas."},"finish_reason":"stop"}],"usage":{"prompt_tokens":20,"completion_tokens":10,"total_tokens":30}}))
        }));
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .unwrap();
}
