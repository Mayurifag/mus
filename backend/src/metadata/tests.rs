use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use axum::{http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use serde_json::json;

use super::*;

fn test_config(provider: Provider) -> Config {
    Config {
        provider,
        api_key: "test-secret-never-log".into(),
        model: match provider {
            Provider::OpenRouter => "openrouter/free",
            Provider::Gemini => "models/gemini-flash-latest",
        }
        .into(),
    }
}

#[test]
fn provider_selection_prefers_openrouter_and_keeps_gemini_optional() {
    let both = config(|key| match key {
        "OPENROUTER_API_KEY" => Some("openrouter-test".into()),
        "GEMINI_API_KEY" => Some("gemini-test".into()),
        _ => None,
    })
    .unwrap();
    assert_eq!(both.provider, Provider::OpenRouter);
    assert_eq!(both.model, "openrouter/free");
    let gemini = config(|key| match key {
        "OPENROUTER_API_KEY" => Some("  ".into()),
        "GEMINI_API_KEY" => Some("gemini-test".into()),
        _ => None,
    })
    .unwrap();
    assert_eq!(gemini.provider, Provider::Gemini);
    assert_eq!(gemini.model, "gemini-flash-latest");
    assert!(config(|_| None).is_none());
}

#[test]
fn model_is_configurable() {
    let selected = config(|key| match key {
        "OPENROUTER_API_KEY" => Some("test-key".into()),
        "OPENROUTER_MODEL" => Some("example/model:free".into()),
        _ => None,
    })
    .unwrap();
    assert_eq!(selected.model, "example/model:free");
}

#[test]
fn metadata_ai_input_has_bounded_size() {
    assert!(!ai_input_exceeds_limits("Song", "Artist"));
    assert!(ai_input_exceeds_limits(
        &"x".repeat(MAX_AI_TITLE_BYTES + 1),
        "Artist"
    ));
    assert!(ai_input_exceeds_limits(
        "Song",
        &"x".repeat(MAX_AI_CHANNEL_BYTES + 1)
    ));
}

#[test]
fn requests_keep_keys_out_of_urls_bodies_and_debug_output() {
    for provider in [Provider::Gemini, Provider::OpenRouter] {
        let config = test_config(provider);
        let request = build_request(&Client::new(), &config, "Song", "Channel").unwrap();
        assert!(request.url().query().is_none());
        assert!(!request.url().as_str().contains(&config.api_key));
        assert!(!format!("{request:?}").contains(&config.api_key));
        let body: Value =
            serde_json::from_slice(request.body().unwrap().as_bytes().unwrap()).unwrap();
        assert!(!body.to_string().contains(&config.api_key));
        let header_name = match provider {
            Provider::Gemini => "x-goog-api-key",
            Provider::OpenRouter => "authorization",
        };
        assert!(request.headers()[header_name].is_sensitive());
        assert!(request.headers()[header_name]
            .to_str()
            .unwrap()
            .contains(&config.api_key));
        match provider {
            Provider::OpenRouter => {
                assert_eq!(body["model"], "openrouter/free");
                assert_eq!(body["provider"]["require_parameters"], true);
                assert_eq!(body["response_format"]["type"], "json_schema");
                assert_eq!(body["max_tokens"], 2048);
            }
            Provider::Gemini => {
                assert!(request
                    .url()
                    .path()
                    .ends_with("gemini-flash-latest:generateContent"));
                assert_eq!(
                    body["generationConfig"]["responseMimeType"],
                    "application/json"
                );
            }
        }
        assert!(body.to_string().contains("Channel"));
        assert!(body.to_string().contains("Song"));
    }
}

#[test]
fn invalid_header_configuration_does_not_echo_secret() {
    let mut config = test_config(Provider::Gemini);
    config.api_key.push_str("\nsecret-suffix");
    let error = build_request(&Client::new(), &config, "Song", "Channel").unwrap_err();
    assert_eq!(error.to_string(), "Invalid metadata API key configuration");
}

#[test]
fn both_providers_parse_metadata_and_filter_untrusted_tags() {
    let text = json!({
        "artist": "Anima, Sheera",
        "title": "Moon",
        "tags": ["cover", "injected-tag", "cover", "ost"]
    })
    .to_string();
    for (provider, response) in [
        (
            Provider::OpenRouter,
            json!({"choices": [{"message": {"content": text}}]}),
        ),
        (
            Provider::Gemini,
            json!({"candidates": [{"content": {"parts": [{"text": text}]}}]}),
        ),
    ] {
        assert_eq!(
            parse_response(provider, &response).unwrap(),
            TrackMetadata {
                artist: "Anima, Sheera".into(),
                title: "Moon".into(),
                tags: vec!["cover".into(), "ost".into()]
            }
        );
    }
}

#[test]
fn invalid_responses_are_rejected_without_echoing_contents() {
    for text in [
        "test-secret-never-log",
        "{\"artist\":\"\",\"title\":\"Song\"}",
        "{\"artist\":\"Artist\",\"title\":\" \"}",
        "{\"artist\":\"Artist\"}",
    ] {
        let response = json!({"choices": [{"message": {"content": text}}]});
        let error = parse_response(Provider::OpenRouter, &response).unwrap_err();
        assert!(!error.to_string().contains("test-secret-never-log"));
    }
    for response in [
        json!({}),
        json!({"choices": []}),
        json!({"error": {"message": "secret"}}),
    ] {
        assert!(parse_response(Provider::OpenRouter, &response).is_err());
    }
}

async fn mock_request(status: StatusCode, retry_success: bool) -> (Result<Value>, usize) {
    let count = Arc::new(AtomicUsize::new(0));
    let calls = count.clone();
    let router = Router::new().route(
        "/",
        post(move || {
            let calls = calls.clone();
            async move {
                let attempt = calls.fetch_add(1, Ordering::SeqCst);
                if retry_success && attempt > 0 {
                    Json(json!({"ok": true})).into_response()
                } else {
                    (status, "upstream body includes test-secret-never-log").into_response()
                }
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = Client::builder().no_proxy().build().unwrap();
    let mut request = build_request(
        &client,
        &test_config(Provider::OpenRouter),
        "Song",
        "Channel",
    )
    .unwrap();
    *request.url_mut() = url.parse().unwrap();
    let result = send_request(&client, request).await;
    server.abort();
    (result, count.load(Ordering::SeqCst))
}

#[tokio::test]
async fn transient_server_error_is_retried_once() {
    let (result, count) = mock_request(StatusCode::SERVICE_UNAVAILABLE, true).await;
    assert_eq!(result.unwrap(), json!({"ok": true}));
    assert_eq!(count, 2);
    let (result, count) = mock_request(StatusCode::SERVICE_UNAVAILABLE, false).await;
    assert_eq!(
        result.unwrap_err().to_string(),
        "Metadata service returned HTTP 503"
    );
    assert_eq!(count, 2);
}

#[tokio::test]
async fn auth_rate_limit_and_bad_json_fail_safely_without_retry() {
    for status in [
        StatusCode::UNAUTHORIZED,
        StatusCode::TOO_MANY_REQUESTS,
        StatusCode::OK,
    ] {
        let (result, count) = mock_request(status, false).await;
        let error = result.unwrap_err().to_string();
        assert!(!error.contains("test-secret-never-log"));
        assert_eq!(count, 1);
    }
}
