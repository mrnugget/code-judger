use anyhow::Result;
use reqwest;
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<()> {
    let mut code = String::from("```");
    code.push_str(include_str!("../data/thorstenball.com.html"));
    code.push_str("```");

    let assertions = vec![
        "[MUST] The year of the copyright notice has to be 2025.",
        "[MUST] The link to the Twitter profile has to be to @thorstenball",
        "Menu item linking to Register Spill must be marked as new",
        "Should mention that Thorsten is happy to receive emails",
        "Has photo of Thorsten",
        "[MUST] Lists his favorite dogs",
    ];

    let formatted_assertions = assertions
        .iter()
        .map(|a| format!("- {}", a))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = include_str!("../prompts/eval.md")
        .replace("<code>", &code)
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

    let result: Value = response.json().await?;
    if let Some(content) = result["content"][0]["text"].as_str() {
        println!("Evaluation result (took {:?}):\n{}", duration, content);
    } else {
        println!("Error: Unexpected response format");
    }

    Ok(())
}
