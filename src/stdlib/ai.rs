// FLUXIS — stdlib/ai.rs
// AI/LLM standard library.
// Supports any OpenAI-compatible API (OpenAI, Groq, Together, Mistral, Ollama, Anthropic, etc.)
//
// Usage:
//   ai_set_key("sk-...")                     — set API key
//   ai_set_url("https://api.openai.com")     — set base URL (default: OpenAI)
//   ai_set_model("gpt-4o")                   — set default model
//   ai_ask("prompt")                          — simple completion
//   ai_chat(history, "new message")           — multi-turn chat
//   ai_model("model", "prompt")               — one-shot with specific model
//
// Preset providers:
//   ai_use("openai")   → api.openai.com      default model: gpt-4o-mini
//   ai_use("groq")     → api.groq.com        default model: llama-3.1-8b-instant
//   ai_use("together") → api.together.ai     default model: meta-llama/Llama-3-8b
//   ai_use("mistral")  → api.mistral.ai      default model: mistral-small-latest
//   ai_use("anthropic")→ api.anthropic.com   default model: claude-haiku-4-5-20251001
//   ai_use("ollama")   → localhost:11434     default model: llama3

use crate::vm::value::Value;
use crate::error::{FluxisError, runtime_error, type_error, arity_error};

// ── PROVIDER STATE ────────────────────────────────────────────────────────
fn get_key()   -> String { std::env::var("FLUXIS_AI_KEY").unwrap_or_default() }
fn get_url()   -> String { std::env::var("FLUXIS_AI_URL").unwrap_or_else(|_| "https://api.openai.com".to_string()) }
fn get_model() -> String { std::env::var("FLUXIS_AI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string()) }

fn set_env(key: &str, val: &str) { unsafe { std::env::set_var(key, val); } }

// ── PROVIDER PRESETS ──────────────────────────────────────────────────────
fn apply_preset(provider: &str) -> Result<(), FluxisError> {
    match provider {
        "openai" => {
            set_env("FLUXIS_AI_URL",   "https://api.openai.com");
            set_env("FLUXIS_AI_MODEL", "gpt-4o-mini");
        }
        "groq" => {
            set_env("FLUXIS_AI_URL",   "https://api.groq.com");
            set_env("FLUXIS_AI_MODEL", "llama-3.1-8b-instant");
        }
        "together" => {
            set_env("FLUXIS_AI_URL",   "https://api.together.ai");
            set_env("FLUXIS_AI_MODEL", "meta-llama/Llama-3-8b-hf");
        }
        "mistral" => {
            set_env("FLUXIS_AI_URL",   "https://api.mistral.ai");
            set_env("FLUXIS_AI_MODEL", "mistral-small-latest");
        }
        "anthropic" => {
            set_env("FLUXIS_AI_URL",   "https://api.anthropic.com");
            set_env("FLUXIS_AI_MODEL", "claude-haiku-4-5-20251001");
        }
        "ollama" => {
            set_env("FLUXIS_AI_URL",   "http://localhost:11434");
            set_env("FLUXIS_AI_MODEL", "llama3");
        }
        other => return Err(runtime_error(&format!(
            "Unknown provider '{}'. Use: openai, groq, together, mistral, anthropic, ollama", other
        )).with_hint("Or set manually: ai_set_url(url) + ai_set_model(model)")),
    }
    Ok(())
}

// ── API CALL ──────────────────────────────────────────────────────────────
fn call_api(messages_json: &str, model: &str) -> Result<String, FluxisError> {
    use std::process::Command;

    let key   = get_key();
    let url   = get_url();

    // Build the request body (OpenAI-compatible format)
    let body = format!(
        r#"{{"model":"{}","messages":{}}}"#,
        model, messages_json
    );

    // Anthropic needs different headers and endpoint
    let is_anthropic = url.contains("anthropic.com");

    let mut cmd = Command::new("curl");
    cmd.args(["-s", "-X", "POST"]);

    if is_anthropic {
        cmd.arg(format!("{}/v1/messages", url));
        cmd.args(["-H", "content-type: application/json"]);
        cmd.args(["-H", &format!("x-api-key: {}", key)]);
        cmd.args(["-H", "anthropic-version: 2023-06-01"]);
        // Anthropic body needs max_tokens
        let anthropic_body = format!(
            r#"{{"model":"{}","max_tokens":1024,"messages":{}}}"#,
            model, messages_json
        );
        cmd.args(["-d", &anthropic_body]);
    } else {
        cmd.arg(format!("{}/v1/chat/completions", url));
        cmd.args(["-H", "content-type: application/json"]);
        if !key.is_empty() {
            cmd.args(["-H", &format!("Authorization: Bearer {}", key)]);
        }
        cmd.args(["-d", &body]);
    }

    let out = cmd.output()
        .map_err(|e| runtime_error(&format!("curl failed: {}. Install with: pkg install curl", e)))?;

    let resp = String::from_utf8_lossy(&out.stdout).to_string();

    if resp.trim().is_empty() {
        return Err(runtime_error("AI API returned empty response — check your URL and key"));
    }

    if resp.contains("\"error\"") {
        let snippet = &resp[..resp.len().min(300)];
        return Err(runtime_error(&format!("AI API error: {}", snippet))
            .with_hint("Check your API key and model name"));
    }

    Ok(resp)
}

// ── RESPONSE PARSING ──────────────────────────────────────────────────────
fn extract_text(resp: &str) -> String {
    // OpenAI format: choices[0].message.content
    if let Some(pos) = resp.find("\"content\":\"") {
        let after = &resp[pos + 11..];
        if let Some(end) = after.find("\"}") {
            return after[..end]
                .replace("\\n", "\n")
                .replace("\\\"", "\"")
                .replace("\\\\", "\\")
                .replace("\\t", "\t");
        }
    }
    // Fallback: Anthropic format — content[0].text
    if let Some(pos) = resp.find("\"text\":\"") {
        let after = &resp[pos + 8..];
        if let Some(end) = after.find("\"}") {
            return after[..end]
                .replace("\\n", "\n")
                .replace("\\\"", "\"")
                .replace("\\\\", "\\");
        }
    }
    resp.to_string()
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"', "\\\"")
     .replace('\n', "\\n")
     .replace('\t', "\\t")
}

// ── PUBLIC API ────────────────────────────────────────────────────────────
pub fn call(name: &str, args: &[Value]) -> Result<Value, FluxisError> {
    match name {
        "ai_set_key" => {
            if args.len() != 1 { return Err(arity_error("ai_set_key", 1, args.len())); }
            let key = match &args[0] { Value::Str(s) => s.clone(), other => return Err(type_error(&format!("ai_set_key() expects string, got {}", other.type_name()))) };
            set_env("FLUXIS_AI_KEY", &key);
            Ok(Value::Str("API key set".to_string()))
        }

        "ai_set_url" => {
            if args.len() != 1 { return Err(arity_error("ai_set_url", 1, args.len())); }
            let url = match &args[0] { Value::Str(s) => s.trim_end_matches('/').to_string(), other => return Err(type_error(&format!("ai_set_url() expects string, got {}", other.type_name()))) };
            set_env("FLUXIS_AI_URL", &url);
            Ok(Value::Str(format!("API URL set to {}", url)))
        }

        "ai_set_model" => {
            if args.len() != 1 { return Err(arity_error("ai_set_model", 1, args.len())); }
            let model = match &args[0] { Value::Str(s) => s.clone(), other => return Err(type_error(&format!("ai_set_model() expects string, got {}", other.type_name()))) };
            set_env("FLUXIS_AI_MODEL", &model);
            Ok(Value::Str(format!("Model set to {}", model)))
        }

        "ai_use" => {
            if args.len() != 1 { return Err(arity_error("ai_use", 1, args.len())); }
            let provider = match &args[0] { Value::Str(s) => s.to_lowercase(), other => return Err(type_error(&format!("ai_use() expects string, got {}", other.type_name()))) };
            apply_preset(&provider)?;
            Ok(Value::Str(format!("Using {} provider", provider)))
        }

        "ai_ask" => {
            if args.len() != 1 { return Err(arity_error("ai_ask", 1, args.len())); }
            let prompt = match &args[0] { Value::Str(s) => escape(s), other => escape(&other.display()) };
            let model  = get_model();
            let msgs   = format!(r#"[{{"role":"user","content":"{}"}}]"#, prompt);
            let resp   = call_api(&msgs, &model)?;
            Ok(Value::Str(extract_text(&resp)))
        }

        "ai_model" => {
            if args.len() != 2 { return Err(arity_error("ai_model", 2, args.len())); }
            let model  = match &args[0] { Value::Str(s) => s.clone(), other => other.display() };
            let prompt = match &args[1] { Value::Str(s) => escape(s), other => escape(&other.display()) };
            let msgs   = format!(r#"[{{"role":"user","content":"{}"}}]"#, prompt);
            let resp   = call_api(&msgs, &model)?;
            Ok(Value::Str(extract_text(&resp)))
        }

        "ai_chat" => {
            if args.len() != 2 { return Err(arity_error("ai_chat", 2, args.len())); }
            let model   = get_model();
            let new_msg = match &args[1] { Value::Str(s) => escape(s), other => escape(&other.display()) };
            let roles   = ["user", "assistant"];
            let mut msgs = String::from("[");
            if let Value::Array(hist) = &args[0] {
                for (i, v) in hist.iter().enumerate() {
                    if i > 0 { msgs.push(','); }
                    msgs.push_str(&format!(r#"{{"role":"{}","content":"{}"}}"#,
                        roles[i % 2], escape(&v.display())));
                }
                if !hist.is_empty() { msgs.push(','); }
            }
            msgs.push_str(&format!(r#"{{"role":"user","content":"{}"}}"#, new_msg));
            msgs.push(']');
            let resp = call_api(&msgs, &model)?;
            Ok(Value::Str(extract_text(&resp)))
        }

        "ai_get_model" => Ok(Value::Str(get_model())),
        "ai_get_url"   => Ok(Value::Str(get_url())),

        _ => Err(runtime_error(&format!("Unknown ai function '{}'", name))),
    }
}

pub fn is_ai_fn(name: &str) -> bool {
    matches!(name,
        "ai_set_key" | "ai_set_url" | "ai_set_model" | "ai_use" |
        "ai_ask"     | "ai_model"   | "ai_chat"       |
        "ai_get_model" | "ai_get_url"
    )
}

