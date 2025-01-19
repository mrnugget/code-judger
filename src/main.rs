use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize)]
struct ContentItem {
    text: String,
    #[serde(rename = "type")]
    content_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentItem>,
}

#[derive(Debug)]
struct JudgeResult {
    score: f64,
    message: String,
}

async fn judge_code(code: &str, assertions: Vec<&str>) -> Result<JudgeResult> {
    let mut fenced_code = String::from("```");
    fenced_code.push_str(code);
    fenced_code.push_str("```");

    let formatted_assertions = assertions
        .iter()
        .map(|a| format!("- {}", a))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = include_str!("../prompts/eval.md")
        .replace("<code>", &fenced_code)
        .replace("<assertions>", &formatted_assertions);

    let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY is not set");
    let model = "claude-3-5-haiku-20241022";

    let start = std::time::Instant::now();
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&json!({
            "model": model,
            "temperature": 0.0,
            "messages": [{
                "role": "user",
                "content": prompt
            }],
            "max_tokens": 1024
        }))
        .send()
        .await?;
    let duration = start.elapsed();

    let result: ClaudeResponse = response.json().await?;
    println!("Request took: {:?}", duration);

    let (message, score_text) = result.content[0]
        .text
        .rsplit_once('\n')
        .ok_or(anyhow::anyhow!("Failed to parse score"))?;
    let score = score_text.parse::<f64>()?;

    Ok(JudgeResult {
        score,
        message: message.trim().into(),
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let assertions = vec![
        "[MUST] The year of the copyright notice has to be 2025.",
        "[MUST] The link to the Twitter profile has to be to @thorstenball",
        "Menu item linking to Register Spill must be marked as new",
        "Should mention that Thorsten is happy to receive emails",
        "Has photo of Thorsten",
    ];
    let code = include_str!("../data/thorstenball.com.html");

    let result = judge_code(code, assertions).await?;
    let escape_code_red = "\x1b[31m";
    let escape_code_green = "\x1b[32m";
    let escape_code_reset = "\x1b[0m";
    println!(
        "========= Result =======\nMessage: {}\n\nScore: {}{}{}\n",
        result.message,
        if result.score < 2.0 {
            escape_code_red
        } else {
            escape_code_green
        },
        result.score,
        escape_code_reset
    );
    Ok(())
}
