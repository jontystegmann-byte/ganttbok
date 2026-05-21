/// Tiny Telegram Bot API client. Single function — POST a text message to a chat.
///
/// Bot tokens look like `123456:ABC-DEF...`. The chat_id is the user's numeric Telegram ID
/// (can be obtained by having the user message @userinfobot, or by the bot itself when
/// the user sends `/start`).
///
/// Usage:
/// ```
/// telegram::send_message("123456:ABC...", "987654321", "Hello!")?;
/// ```
use serde::Serialize;

const API_BASE: &str = "https://api.telegram.org";

#[derive(Serialize)]
struct SendMessagePayload<'a> {
    chat_id: &'a str,
    text: &'a str,
    /// Parse mode lets us use *bold* / _italic_ / `code` if we want — keep it simple for now.
    parse_mode: &'a str,
}

#[derive(Debug, thiserror::Error)]
pub enum TelegramError {
    #[error("network error: {0}")]
    Net(String),
    #[error("Telegram API error: {0}")]
    Api(String),
    #[error("invalid bot token (must look like 123456:ABC...)")]
    InvalidToken,
}

pub fn send_message(token: &str, chat_id: &str, text: &str) -> Result<(), TelegramError> {
    if token.is_empty() || !token.contains(':') {
        return Err(TelegramError::InvalidToken);
    }
    let url = format!("{API_BASE}/bot{token}/sendMessage");
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| TelegramError::Net(e.to_string()))?;

    let resp = client
        .post(&url)
        .json(&SendMessagePayload { chat_id, text, parse_mode: "Markdown" })
        .send()
        .map_err(|e| TelegramError::Net(e.to_string()))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_else(|_| "<no body>".into());
        return Err(TelegramError::Api(format!("status {status}: {body}")));
    }
    Ok(())
}
