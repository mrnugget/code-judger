use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "anthropic")]
    provider: String,
}

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

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    content: String,
}

struct Judgement {
    score: f64,
    message: String,
}

fn get_claude_response(prompt: &str) -> Result<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY is not set");
    let model = "claude-3-5-haiku-latest";

    let mut response: ClaudeResponse = ureq::post("https://api.anthropic.com/v1/messages")
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
        }))?
        .into_json()?;

    Ok(response.content.remove(0).text)
}

fn get_openai_response(prompt: &str) -> Result<String> {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY is not set");
    let model = "gpt-4o";

    let response: OpenAIResponse = ureq::post("https://api.openai.com/v1/chat/completions")
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("content-type", "application/json")
        .send_json(json!({
            "model": model,
            "temperature": 0.0,
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        }))?
        .into_json()?;

    Ok(response.choices[0].message.content.clone())
}

fn get_llm_response(provider: &str, prompt: &str) -> Result<String> {
    match provider {
        "anthropic" => get_claude_response(prompt),
        "openai" => get_openai_response(prompt),
        _ => Err(anyhow::anyhow!("Unsupported provider: {}", provider)),
    }
}

fn judge_code(provider: &str, code: &str, assertions: &[&str]) -> Result<Judgement> {
    let mut fenced_code = String::from("```");
    fenced_code.push_str(code);
    fenced_code.push_str("```");

    let formatted_assertions = assertions
        .iter()
        .map(|a| format!("- {}", a))
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = include_str!("../prompts/judge.md")
        .replace("<code>", &fenced_code)
        .replace("<assertions>", &formatted_assertions);

    let response = get_llm_response(provider, &prompt)?;

    let (message, score_text) = response
        .rsplit_once('\n')
        .ok_or(anyhow::anyhow!("Failed to parse score"))?;
    let re = regex::Regex::new(r"\d+").unwrap();
    let caps = re
        .captures(score_text)
        .with_context(|| format!("Failed to find score in: {}", score_text))?;
    let score = caps
        .get(0)
        .map(|m| {
            m.as_str()
                .parse::<f64>()
                .with_context(|| format!("Failed to parse score: {}", m.as_str()))
        })
        .transpose()?
        .expect("Failed to parse score");

    Ok(Judgement {
        score,
        message: message.trim().into(),
    })
}

const RED: &'static str = "\x1b[31m";
const GREEN: &'static str = "\x1b[32m";
const RESET: &'static str = "\x1b[0m";

fn main() -> Result<()> {
    let args = Args::parse();

    let code = include_str!("../data/code-to-judge");

    let test_cases = vec![
        vec![
            "[MUST] The year of the copyright notice has to be 2025.",
            "[MUST] The link to the Twitter profile has to be to @thorstenball",
            "Menu item linking to Register Spill must be marked as new",
            "Should mention that Thorsten is happy to receive emails",
            "Has photo of Thorsten",
        ],
        vec![
            "[MUST] The year of the copyright notice has to be 2025.",
            "[MUST] The link to the Twitter profile has to be to @thorstenball",
            "Menu item linking to Register Spill must be marked as new",
            "[MUST] Must mention that Thorsten is happy to phone calls",
            "Has photo of Thorsten",
        ],
    ];

    for assertions in test_cases.iter() {
        let result = judge_code(&args.provider, code, &assertions)?;
        println!(
            "========= Result =======\nMessage: {}\n\nScore: {}{}{}\n",
            result.message,
            if result.score < 2.0 { RED } else { GREEN },
            result.score,
            RESET
        );
    }

    Ok(())
}
