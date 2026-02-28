use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Serialize)]
struct LLMMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct LLMRequest {
    model: String,
    messages: Vec<LLMMessage>,
    stream: bool,
}

#[derive(Deserialize)]
struct LLMResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct LLMResponse {
    response: Option<String>,
    message: Option<LLMResponseMessage>,
}

pub struct LLM {
    client: Client,
    url: String,
}

impl LLM {
    pub fn new() -> Result<Self, String> {
        match env::var("LLM_URL") {
            Ok(url) => Ok(Self {
                client: Client::new(),
                url,
            }),
            Err(_) => Err("Failed to create LLM URL".to_string()),
        }
    }

    pub async fn parse_text(&self, text: String) -> Result<String, String> {
        println!("START PARSING...");
        let today_date = chrono::Local::now().to_string();
        let prompt = format!(
            r#"
Ты финансовый парсер.

Твоя задача: извлечь данные из текста и вернуть СТРОГО JSON.

Правила:
- Ответ должен содержать ТОЛЬКО валидный JSON.
- Без пояснений.
- Без markdown.
- Без комментариев.
- Без дополнительного текста.

Формат ответа:

{{
  "type": "expense | income | income-loan | expense-loan",
  "category": "food | transport | entertainment | salary | loan | ...",
  "item": "banana | loan | shirts | meat | ...",
  "amount": float,
  "currency": "RUB | USD | UZS | ...",
  "date": "date with foramt ISO 8601",
  "with_who": "me | <person name> | he | she | friend | ..."
}}

Если указано "вчера | час назат | прошлом неделю | ..." — рассчитай дату относительно сегодняшнего дня: {today_date}

Текст:
"{text}"
"#
        );

        match self
            .client
            .post(format!("{}/chat", &self.url))
            .json(&LLMRequest {
                model: "qwen2.5:3b".to_string(),
                messages: vec![
                    LLMMessage {
                        role: "user".to_string(),
                        content: prompt
                    },
                    LLMMessage {
                        role: "system".to_string(),
                        content: "Ты финансовый парсер. Отвечай только валидным JSON.Если ты не уверен — всё равно верни JSON с null значениями".to_string()
                    }
                ],
                stream: false,
            })
            .send()
            .await
        {
            Ok(resp) => match resp.text().await {
                Ok(txt) => {
                    let parsed: LLMResponse = serde_json::from_str(&txt)
                        .map_err(|e| format!("Failed to parse json: {}. Body: {}", e, txt))?;

                    if let Some(content) = parsed.message.map(|m| m.content) {
                        Ok(content)
                    } else if let Some(response) = parsed.response {
                        Ok(response)
                    } else {
                        Err(format!(
                            "LLM response has neither `message.content` nor `response`. Body: {}",
                            txt
                        ))
                    }
                }
                Err(e) => Err(format!("Failed to read LLM response body: {}", e)),
            },
            Err(e) => Err(format!("ERROR on setding to LLM {}", e.to_string())),
        }
    }
}
