use std::ops::RangeInclusive;

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
    score: i32,
    message: String,
}

fn get_claude_response(prompt: &str) -> Result<String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY is not set");
    let model = "claude-3-5-sonnet-latest";

    let response = ureq::post("https://api.anthropic.com/v1/messages")
        .set("x-api-key", &api_key)
        .set("anthropic-version", "2023-06-01")
        .set("content-type", "application/json")
        .send_json(json!({
            "model": model,
            "temperature": 0.0,
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": prompt
            }],
        }));

    let response = match response {
        Ok(res) => res,
        Err(ureq::Error::Status(status, res)) => {
            let error_body = res
                .into_string()
                .context("Failed to read error response body")?;
            return Err(anyhow::anyhow!("API error {}: {}", status, error_body));
        }
        Err(e) => return Err(e.into()),
    };

    let mut response_body: ClaudeResponse = response.into_json()?;
    Ok(response_body.content.remove(0).text)
}

fn get_openai_response(prompt: &str) -> Result<String> {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY is not set");
    let model = "gpt-4o";

    let response = ureq::post("https://api.openai.com/v1/chat/completions")
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("content-type", "application/json")
        .send_json(json!({
            "model": model,
            "temperature": 0.0,
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        }));

    let response = match response {
        Ok(res) => res,
        Err(ureq::Error::Status(status, res)) => {
            let error_body = res
                .into_string()
                .context("Failed to read error response body")?;
            return Err(anyhow::anyhow!("API error {}: {}", status, error_body));
        }
        Err(e) => return Err(e.into()),
    };

    let response_body: OpenAIResponse = response.into_json()?;
    Ok(response_body.choices[0].message.content.clone())
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

    let score =
        parse_score(&response).with_context(|| format!("Failed to parse score from response"))?;

    Ok(Judgement {
        score,
        message: response,
    })
}

const VALID_SCORE_RANGE: RangeInclusive<i32> = 1..=3;

fn parse_score(response: &str) -> Result<i32> {
    response
        .lines()
        .last()
        .map(|s| s.trim())
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("Failed to parse score from response:\n{}", response))
        .and_then(|score: i32| {
            if VALID_SCORE_RANGE.contains(&score) {
                Ok(score)
            } else {
                Err(anyhow::anyhow!("Score {} out of valid range 1-3", score))
            }
        })
}

const RED: &'static str = "\x1b[31m";
const GREEN: &'static str = "\x1b[32m";
const RESET: &'static str = "\x1b[0m";

struct TestCase {
    assertions: Vec<&'static str>,
    expected_score: RangeInclusive<i32>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let code = include_str!("../data/code-to-judge");

    let test_cases = vec![
        TestCase {
            assertions: vec![
                "[MUST] The year of the copyright notice has to be 2025.",
                "[MUST] The link to the Twitter profile has to be to @thorstenball (ignore the lack of protocol)",
                "Menu item linking to Register Spill must be marked as new",
                "Should mention that Thorsten is happy to receive emails",
                "Has photo of Thorsten",
            ],
            expected_score: 2..=3,
        },
        TestCase {
            assertions: vec![
                "[MUST] The year of the copyright notice has to be 2025.",
                "[MUST] The link to the Twitter profile has to be to @thorstenball",
                "Menu item linking to Register Spill must be marked as new",
                "[MUST] Must mention that Thorsten is happy to phone calls",
                "Has photo of Thorsten",
            ],
            expected_score: 1..=1,
        },
    ];

    for test_case in test_cases.iter() {
        let result = judge_code(&args.provider, code, &test_case.assertions)?;
        println!(
            "========= Result =======\nMessage: {}\n\nScore: {}{}{}\n",
            result.message,
            if result.score < 2 { RED } else { GREEN },
            result.score,
            RESET
        );

        assert!(
            test_case.expected_score.contains(&result.score),
            "Score {:?} out of expected range {:?}",
            result.score,
            test_case.expected_score
        );
    }

    Ok(())
}
