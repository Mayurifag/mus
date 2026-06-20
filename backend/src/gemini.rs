use std::env;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const DEFAULT_MODEL: &str = "gemini-flash-latest";
const SYSTEM_PROMPT: &str = "Extract artist(s), title, and conservative tags from a YouTube video title. Strip junk words: Official Video/Audio/Music Video, Lyric Video, Lyrics, HD, 4K, HQ, Remastered, Live, Audio Only, Music Video, Fan Made, Visualizer, Topic, Extended Mix, Original Mix, Radio Edit, Album/Single Version, Provided to YouTube, Auto-generated. Multiple artists (feat./ft./featuring/x/&/and) -> comma-separated in 'artist', do NOT include feat./ft./featuring keyword itself but DO keep the featured artist name. Artist comes from the video title first; use channel name only if the title gives no artist. Return tags as an array containing only 'gachi' and/or 'ai-cover' when clearly indicated by title/channel; otherwise return an empty array.";

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct TrackMetadata {
    pub artist: String,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

pub async fn parse_track_metadata(raw_title: &str, channel_name: &str) -> Option<TrackMetadata> {
    let api_key = env::var("GEMINI_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())?;
    let model = env::var("GEMINI_MODEL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.into());
    match request_track_metadata(&api_key, &model, raw_title, channel_name).await {
        Ok(metadata) if !metadata.title.trim().is_empty() => Some(metadata),
        Ok(_) => None,
        Err(error) => {
            tracing::warn!(error = %error, "Gemini request failed, skipping LLM parsing");
            None
        }
    }
}

async fn request_track_metadata(
    api_key: &str,
    model: &str,
    raw_title: &str,
    channel_name: &str,
) -> Result<TrackMetadata> {
    let model = model.trim_start_matches("models/");
    let url =
        format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent");
    let response: GenerateContentResponse = reqwest::Client::new()
        .post(url)
        .query(&[("key", api_key)])
        .json(&build_request(raw_title, channel_name))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    parse_response(&response)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateContentRequest {
    contents: Vec<Content>,
    system_instruction: SystemInstruction,
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Serialize, Deserialize)]
struct Content {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    #[serde(default)]
    parts: Vec<Part>,
}

#[derive(Serialize, Deserialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    response_mime_type: &'static str,
    response_schema: Value,
    max_output_tokens: u16,
}

#[derive(Deserialize)]
struct GenerateContentResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Content,
}

fn build_request(raw_title: &str, channel_name: &str) -> GenerateContentRequest {
    let mut user_message = format!("Video title: \"{raw_title}\"");
    if !channel_name.is_empty() {
        user_message.push_str(&format!("\nChannel name: \"{channel_name}\""));
    }
    GenerateContentRequest {
        contents: vec![Content {
            role: Some("user".into()),
            parts: vec![Part { text: user_message }],
        }],
        system_instruction: SystemInstruction {
            parts: vec![Part {
                text: SYSTEM_PROMPT.into(),
            }],
        },
        generation_config: GenerationConfig {
            response_mime_type: "application/json",
            response_schema: json!({
                "type": "OBJECT",
                "properties": {
                    "artist": { "type": "STRING" },
                    "title": { "type": "STRING" },
                    "tags": {
                        "type": "ARRAY",
                        "items": { "type": "STRING", "enum": ["gachi", "ai-cover"] }
                    }
                },
                "required": ["artist", "title", "tags"]
            }),
            max_output_tokens: 2048,
        },
    }
}

fn parse_response(response: &GenerateContentResponse) -> Result<TrackMetadata> {
    let text = response
        .candidates
        .first()
        .and_then(|candidate| candidate.content.parts.first())
        .map(|part| part.text.as_str())
        .context("Gemini response missing text")?;
    Ok(serde_json::from_str(text)?)
}

#[cfg(test)]
mod tests {
    use super::{
        build_request, parse_response, Candidate, Content, GenerateContentResponse, Part,
        TrackMetadata,
    };

    #[test]
    fn build_request_includes_channel_when_present() {
        let request = build_request("Anima Ft. Sheera - Moon (Original Mix)", "AnimaChannel");
        let text = &request.contents[0].parts[0].text;
        assert!(text.contains("Video title: \"Anima Ft. Sheera - Moon (Original Mix)\""));
        assert!(text.contains("Channel name: \"AnimaChannel\""));
        assert_eq!(
            request.generation_config.response_mime_type,
            "application/json"
        );
    }

    #[test]
    fn parse_response_extracts_metadata() {
        let response = GenerateContentResponse {
            candidates: vec![Candidate {
                content: Content {
                    role: None,
                    parts: vec![Part {
                        text: "{\"artist\":\"Anima, Sheera\",\"title\":\"Moon\"}".into(),
                    }],
                },
            }],
        };
        assert_eq!(
            parse_response(&response).unwrap(),
            TrackMetadata {
                artist: "Anima, Sheera".into(),
                title: "Moon".into(),
                tags: Vec::new(),
            }
        );
    }
}
