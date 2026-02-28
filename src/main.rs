mod llm;
mod tg_bot;

use crate::llm::LLM;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::sync::Arc;
use tg_bot::{TelegramBot, Update};

enum AppError {
    InvalidOperation(String),
}

#[derive(Serialize)]
struct MsgError {
    error: String,
}

#[derive(Serialize)]
struct MsgOk {
    status: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AppError::InvalidOperation(error) => (StatusCode::BAD_REQUEST, error.to_string()),
        };

        (
            status,
            Json(MsgError {
                error: error_message,
            }),
        )
            .into_response()
    }
}

struct AppState {
    bot: TelegramBot,
    llm: LLM,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let bot = TelegramBot::new().expect("Failed to create bot");
    let llm = LLM::new().expect("Failed to create llm");

    bot.set_webhook().await.unwrap();

    let shared_state = Arc::new(AppState { bot, llm });
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/webhook", post(webhook))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn webhook(
    State(state): State<Arc<AppState>>,
    Json(update): Json<Update>,
) -> Result<Json<MsgOk>, AppError> {
    if let Some(msg) = update.message {
        let chat_id = msg.chat.id;
        let state_for_typing = Arc::clone(&state);
        let typing_handler = tokio::spawn(async move {
            state_for_typing.bot.loop_typing_effect(chat_id).await;
        });
        let mut response = String::new();

        if let Some(text) = msg.text {
            response.push_str(format!("text: {}\n", text).as_str());

            match state.llm.parse_text(text).await {
                Ok(t) => {
                    response.push_str(format!("json: {:#?}\n", t).as_str());
                }
                Err(e) => {
                    response.push_str(format!("error: {}\n", e).as_str());
                }
            }
        }

        if let Some(user) = msg.from {
            response.push_str(format!("from: {}\n", user.first_name).as_str());
        }
        
        // stop typing effect
        typing_handler.abort();

        match state.bot.send_message(chat_id, response).await {
            Ok(_) => Ok(Json(MsgOk {
                status: "Ok".to_string(),
            })),
            Err(_) => Err(AppError::InvalidOperation("SEND_MSG_ERROR".to_string())),
        }
    } else {
        Ok(Json(MsgOk {
            status: "No message".to_string(),
        }))
    }
}
