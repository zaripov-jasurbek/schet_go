use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use tokio::time::{sleep, Duration};

#[derive(Deserialize)]
pub struct Update {
    pub message: Option<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub message_id: i64,
    pub from: Option<User>,
    pub chat: Chat,
    pub text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub language_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chat {
    pub id: i64,
    #[serde(rename = "type")]
    pub chat_type: String,
}

#[derive(Debug, Serialize)]
pub struct SendMessage {
    pub chat_id: i64,
    pub text: String,
}

#[derive(Serialize )]
pub struct TypingEffect {
    chat_id: i64,
    action: String,
}

pub struct TelegramBot {
    client: Client,
    api_url: String,
}

impl TelegramBot {
    pub fn new() -> Result<Self, String> {
        if let Ok(token) = env::var("BOT_TOKEN") {
            let api_url = format!("https://api.telegram.org/bot{}", token);
            Ok(Self {
                client: Client::new(),
                api_url,
            })
        } else {
            Err("BOT_TOKEN not set".to_string())
        }
    }

    pub async fn set_webhook(&self) -> Result<(), String> {
        let webhook_url = env::var("WEBHOOK_URL");
        if let Ok(webhook_url) = webhook_url {
            let url = format!("{}/setWebhook?url={}", self.api_url, webhook_url);
            match self.client.get(&url).send().await {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to set webhook: {}", e)),
            }
        } else {
            Err("Invalid webhook url".to_string())
        }
    }

    pub async fn send_message(&self, chat_id: i64, text: String) -> Result<(), String> {
        let url = format!("{}/sendMessage", self.api_url);
        let message = SendMessage { chat_id, text };

        match self.client.post(&url).json(&message).send().await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to send message: {}", e)),
        }
    }
    
    pub async fn typing_effect(&self,chat_id:i64) -> Result<(), String> {
        let url = format!("{}/sendChatAction", self.api_url);
        
        match self.client.post(&url).json(&TypingEffect {
            chat_id,
            action: "typing".to_string(),
        }).send().await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to typing message: {}", e)),
        }
    }
    
    pub async fn loop_typing_effect(&self, chat_id:i64) {
        loop {
            self.typing_effect(chat_id).await.expect("Typing effect failed");
            sleep(Duration::from_secs(4)).await;
        }
    }
}
