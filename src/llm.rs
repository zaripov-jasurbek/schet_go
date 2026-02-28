use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Serialize, Deserialize, Debug)]
pub struct LLMContent {
    #[serde(rename = "type")]
    type_ctx: Option<String>,
    category: Option<String>,
    item: Option<String>,
    amount: Option<f64>,
    currency: Option<String>,
    date: Option<String>,
    person: Option<String>,
}

#[derive(Serialize)]
struct LLMMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct LLMOptions {
    temperature: u8,
}

#[derive(Serialize)]
struct LLMRequest {
    model: String,
    messages: Vec<LLMMessage>,
    format: String,
    options: LLMOptions,
    stream: bool,
}

#[derive(Deserialize)]
struct LLMResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct LLMResponse {
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

    pub async fn parse_text(&self, text: String) -> Result<LLMContent, String> {
        let today_date = chrono::Local::now().to_string();
        let prompt = format!(
            r#"
Ты финансовый парсер.Твоя задача: извлечь данные из текста и вернуть СТРОГО JSON.
Правила:
- Ответ должен содержать ТОЛЬКО валидный JSON.
- Без пояснений.
- Без markdown.
- Без комментариев.
- Без дополнительного текста.
- Поле "date" только в UTC (суффикс Z), пример: 2026-02-17T19:00:00Z и если найдёш слово связано с времям рассчитай дату относительно сегодняшнего дня: {today_date}
Формат ответа:
{{
  "type": "expense | income | income-loan | expense-loan",
  "category": "food | transport | entertainment | salary | loan | ...",
  "item": "banana | loan | shirts | meat | ...",
  "amount": float,
  "currency": "RUB | USD | UZS | ...",
  "date": "date with foramt ISO 8601",
  "person": "я | он | слово который указиваеть на когото но на англиском |me | person name | he | friend | ..."
}}

Текст:
{text}
"#
        );

        match self
            .client
            .post(format!("{}/chat", &self.url))
            .json(&LLMRequest {
                model: "qwen2.5:3b".to_string(),
                messages: vec![
                    LLMMessage {
                        role: "system".to_string(),
                        content: "Ты финансовый парсер. Отвечай только валидным JSON.Если ты не уверен — всё равно верни JSON с null значениями".to_string()
                    },
                    LLMMessage {
                        role: "user".to_string(),
                        content: prompt
                    },
                ],
                format: "json".to_string(),
                options: LLMOptions { temperature: 0 },
                stream: false,
            })
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if !status.is_success() {
                    return match resp.text().await {
                        Ok(body) => Err(format!("LLM HTTP error {}. Body: {}", status, body)),
                        Err(e) => Err(format!(
                            "LLM HTTP error {} and failed to read body: {}",
                            status,
                            e
                        )),
                    };
                }

                match resp.json::<LLMResponse>().await {
                    Ok(parsed) => {
                        match parsed.message.map(|m| m.content) {
                            Some(content) => {
                                match serde_json::from_str::<LLMContent>(content.as_str()) {
                                    Ok(message) => Ok(message),
                                    Err(e) => Err(format!("LLM JSON parse error: {}", e)),
                                }
                            }
                            None => Err("Failed to parse LLM response".to_string()),
                        }
                    }
                    Err(e) => Err(format!("Failed to parse LLM JSON body: {}", e)),
                }
            }
            Err(e) => Err(format!("ERROR on setding to LLM {}", e.to_string())),
        }
    }
}
