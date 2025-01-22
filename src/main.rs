use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

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
struct Judgement {
    score: f64,
    message: String,
}

fn judge_code(code: &str, assertions: Vec<&str>) -> Result<Judgement> {
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

    let response: ClaudeResponse = ureq::post("https://api.anthropic.com/v1/messages")
        .set("x-api-key", &api_key)
        .set("anthropic-version", "2023-06-01")
        .set("content-type", "application/json")
        .send_json(json!({
            "model": model,
            "temperature": 0.0,
            "messages": [{
                "role": "user",
                "content": prompt
            }],
            "max_tokens": 1024
        }))?
        .into_json()?;

    let duration = start.elapsed();
    println!("Request took: {:?}", duration);

    let (message, score_text) = response.content[0]
        .text
        .rsplit_once('\n')
        .ok_or(anyhow::anyhow!("Failed to parse score"))?;
    let score = score_text.parse::<f64>()?;

    Ok(Judgement {
        score,
        message: message.trim().into(),
    })
}

fn main() -> Result<()> {
    let assertions = vec![
        "[MUST] The year of the copyright notice has to be 2025.",
        "[MUST] The link to the Twitter profile has to be to @thorstenball",
        "Menu item linking to Register Spill must be marked as new",
        "Should mention that Thorsten is happy to receive emails",
        "Has photo of Thorsten",
    ];
    let code = include_str!("../data/code-to-judge");

    let result = judge_code(code, assertions)?;
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
