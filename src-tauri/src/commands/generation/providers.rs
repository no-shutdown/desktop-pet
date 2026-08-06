use crate::commands::generation::types::ProviderConfig;
use base64::Engine as _;
use serde_json::Value;
use std::{net::IpAddr, str::FromStr, time::Duration};

const SILICONFLOW_ENDPOINT: &str = "https://api.siliconflow.cn/v1/images/generations";
const OPERATION_TIMEOUT: Duration = Duration::from_secs(120);
const REQUEST_TIMEOUT: Duration = OPERATION_TIMEOUT;
const MAX_IMAGE_BYTES: usize = 16 * 1024 * 1024;
const MAX_ERROR_DETAIL_BYTES: usize = 512;
const LOCAL_SD_NEGATIVE_PROMPT: &str = "ugly, blurry, watermark, multiple characters";
const DEFAULT_DENOISING_STRENGTH: f32 = 0.55;
const MIN_DENOISING_STRENGTH: f32 = 0.35;
const MAX_DENOISING_STRENGTH: f32 = 0.75;

pub fn clamp_denoising_strength(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(MIN_DENOISING_STRENGTH, MAX_DENOISING_STRENGTH)
    } else {
        DEFAULT_DENOISING_STRENGTH
    }
}

pub fn siliconflow_base_body(model: &str, prompt: &str, width: u32, height: u32) -> Value {
    serde_json::json!({
        "model": model,
        "prompt": prompt,
        "image_size": format!("{width}x{height}"),
        "num_inference_steps": 20,
        "num_images": 1,
    })
}

pub fn siliconflow_row_body(
    model: &str,
    prompt: &str,
    image_data_url: &str,
    width: u32,
    height: u32,
) -> Value {
    serde_json::json!({
        "model": model,
        "prompt": prompt,
        "image": image_data_url,
        "image_size": format!("{width}x{height}"),
        "num_inference_steps": 20,
        "num_images": 1,
    })
}

pub fn local_sd_base_body(prompt: &str, width: u32, height: u32) -> Value {
    serde_json::json!({
        "prompt": prompt,
        "negative_prompt": LOCAL_SD_NEGATIVE_PROMPT,
        "steps": 20,
        "width": width,
        "height": height,
        "batch_size": 1,
    })
}

pub fn local_sd_row_body(
    prompt: &str,
    init_image: &str,
    width: u32,
    height: u32,
    denoising_strength: f32,
) -> Value {
    serde_json::json!({
        "prompt": prompt,
        "negative_prompt": LOCAL_SD_NEGATIVE_PROMPT,
        "steps": 20,
        "width": width,
        "height": height,
        "batch_size": 1,
        "init_images": [init_image],
        "denoising_strength": clamp_denoising_strength(denoising_strength),
    })
}

fn is_valid_image_subtype(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '.' | '-')
        })
}

fn canonical_data_url_payload(value: &str) -> Result<&str, String> {
    let value = value.trim();
    let (metadata, payload) = value
        .split_once(',')
        .ok_or_else(|| "canonical Base image must be a complete data URL".to_string())?;
    let metadata = metadata.to_ascii_lowercase();
    let image_metadata = metadata
        .strip_prefix("data:image/")
        .ok_or_else(|| "canonical Base image must use an image data URL".to_string())?;
    let (subtype, encoding) = image_metadata
        .split_once(';')
        .ok_or_else(|| "canonical Base image data URL must use base64".to_string())?;
    if !is_valid_image_subtype(subtype) || encoding != "base64" {
        return Err("canonical Base image data URL metadata is invalid".to_string());
    }

    let payload = payload.trim();
    if payload.is_empty() {
        return Err("canonical Base image data URL has an empty payload".to_string());
    }
    Ok(payload)
}

fn validate_canonical_data_url(value: &str) -> Result<String, String> {
    let value = value.trim();
    let payload = canonical_data_url_payload(value)?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|_| "canonical Base image data URL has invalid base64".to_string())?;
    if decoded.is_empty() {
        return Err("canonical Base image data URL decodes to an empty image".to_string());
    }
    Ok(value.to_string())
}

fn is_blocked_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            address.is_loopback()
                || address.is_private()
                || address.is_link_local()
                || address.is_unspecified()
                || address.is_broadcast()
        }
        IpAddr::V6(address) => {
            if address.is_loopback()
                || address.is_unique_local()
                || address.is_unicast_link_local()
                || address.is_unspecified()
            {
                return true;
            }
            address
                .to_ipv4_mapped()
                .map(|mapped| is_blocked_ip(IpAddr::V4(mapped)))
                .unwrap_or(false)
        }
    }
}

fn validate_image_download_url(value: &str) -> Result<reqwest::Url, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("SiliconFlow image URL is empty".to_string());
    }
    if !value
        .get(..8)
        .map(|prefix| prefix.eq_ignore_ascii_case("https://"))
        .unwrap_or(false)
    {
        return Err("SiliconFlow image URL must use HTTPS".to_string());
    }
    let authority = value[8..]
        .split(|character| matches!(character, '/' | '?' | '#'))
        .next()
        .unwrap_or_default();
    if authority.is_empty() {
        return Err("SiliconFlow image URL must have a non-empty host".to_string());
    }

    let url =
        reqwest::Url::parse(value).map_err(|_| "SiliconFlow image URL is malformed".to_string())?;
    if url.scheme() != "https" {
        return Err("SiliconFlow image URL must use HTTPS".to_string());
    }
    if url.username() != "" || url.password().is_some() {
        return Err("SiliconFlow image URL credentials are not allowed".to_string());
    }

    let host = url
        .host_str()
        .filter(|host| !host.trim().is_empty())
        .ok_or_else(|| "SiliconFlow image URL must have a non-empty host".to_string())?;
    let normalized_host = host.trim_end_matches('.').to_ascii_lowercase();
    if normalized_host == "localhost"
        || normalized_host == "localhost.localdomain"
        || normalized_host.ends_with(".localhost")
    {
        return Err("SiliconFlow image URL host is not allowed".to_string());
    }
    let ip_host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    if IpAddr::from_str(ip_host)
        .map(is_blocked_ip)
        .unwrap_or(false)
    {
        return Err("SiliconFlow image URL host is not allowed".to_string());
    }

    Ok(url)
}

#[derive(Debug, PartialEq, Eq)]
enum ImageSource {
    Bytes(Vec<u8>),
    Url(String),
}

fn local_sd_endpoint(base_url: &str, method: &str) -> String {
    format!(
        "{}/sdapi/v1/{method}",
        base_url.trim().trim_end_matches('/')
    )
}

fn decode_image_data(value: &str, provider: &str) -> Result<Vec<u8>, String> {
    let value = value.trim();
    let encoded = if value.to_ascii_lowercase().starts_with("data:") {
        canonical_data_url_payload(value).map_err(|error| format!("{provider}: {error}"))?
    } else {
        value
    };

    if encoded.is_empty() {
        return Err(format!("{provider}: image base64 is empty"));
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| format!("{provider} base64 decode error: {error}"))?;
    if bytes.is_empty() {
        return Err(format!("{provider}: decoded image is empty"));
    }
    Ok(bytes)
}

fn validate_content_length(content_length: Option<u64>) -> Result<(), String> {
    match content_length {
        Some(0) => Err("SiliconFlow image response body is empty".to_string()),
        Some(length) if length > MAX_IMAGE_BYTES as u64 => Err(format!(
            "SiliconFlow image response exceeds the {} MiB limit",
            MAX_IMAGE_BYTES / (1024 * 1024)
        )),
        _ => Ok(()),
    }
}

fn append_bounded_image_chunk(body: &mut Vec<u8>, chunk: &[u8]) -> Result<(), String> {
    let next_length = body
        .len()
        .checked_add(chunk.len())
        .ok_or_else(|| "SiliconFlow image response is too large".to_string())?;
    if next_length > MAX_IMAGE_BYTES {
        return Err(format!(
            "SiliconFlow image response exceeds the {} MiB limit",
            MAX_IMAGE_BYTES / (1024 * 1024)
        ));
    }
    body.extend_from_slice(chunk);
    Ok(())
}

fn finish_image_body(body: Vec<u8>) -> Result<Vec<u8>, String> {
    if body.is_empty() {
        return Err("SiliconFlow image response body is empty".to_string());
    }
    Ok(body)
}

fn next_http_url_start(value: &str) -> Option<usize> {
    let http = value.find("http://");
    let https = value.find("https://");
    match (http, https) {
        (Some(http), Some(https)) => Some(http.min(https)),
        (Some(http), None) => Some(http),
        (None, Some(https)) => Some(https),
        (None, None) => None,
    }
}

fn redact_http_urls(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut cursor = 0;
    while cursor < value.len() {
        let remaining = &value[cursor..];
        let Some(relative_start) = next_http_url_start(remaining) else {
            result.push_str(remaining);
            break;
        };
        let start = cursor + relative_start;
        result.push_str(&value[cursor..start]);
        let url_end = value[start..]
            .char_indices()
            .skip(1)
            .find(|(_, character)| {
                character.is_whitespace()
                    || matches!(character, '"' | '\'' | '<' | '>' | ')' | ']' | '}' | ',')
            })
            .map(|(offset, _)| start + offset)
            .unwrap_or(value.len());
        result.push_str("[redacted-url]");
        cursor = url_end;
    }
    result
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

fn sanitize_provider_error_detail(detail: &str) -> String {
    let redacted = redact_http_urls(detail);
    let sanitized: String = redacted
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect();
    truncate_utf8(&sanitized, MAX_ERROR_DETAIL_BYTES)
}

fn sanitize_provider_error_detail_with_secret(detail: &str, secret: Option<&str>) -> String {
    let redacted_secret = secret
        .filter(|secret| !secret.is_empty())
        .map(|secret| detail.replace(secret, "[redacted-secret]"))
        .unwrap_or_else(|| detail.to_string());
    sanitize_provider_error_detail(&redacted_secret)
}

fn bounded_sanitized_error(prefix: &str, detail: &str, secret: Option<&str>) -> String {
    let detail = sanitize_provider_error_detail_with_secret(detail, secret);
    if detail.is_empty() {
        return prefix.to_string();
    }
    truncate_utf8(&format!("{prefix}: {detail}"), MAX_ERROR_DETAIL_BYTES)
}

fn first_image<'a>(response: &'a Value, provider: &str) -> Result<&'a Value, String> {
    response
        .get("images")
        .and_then(Value::as_array)
        .and_then(|images| images.first())
        .ok_or_else(|| format!("{provider}: missing image in response"))
}

fn siliconflow_image_source(response: &Value) -> Result<ImageSource, String> {
    let image = first_image(response, "SiliconFlow")?;

    if let Some(encoded) = image.get("b64_json") {
        if let Some(encoded) = encoded.as_str() {
            return decode_image_data(encoded, "SiliconFlow").map(ImageSource::Bytes);
        }
        if !encoded.is_null() {
            return Err("SiliconFlow: invalid b64_json image field".to_string());
        }
    }

    if let Some(url) = image.get("url").and_then(Value::as_str) {
        if !url.trim().is_empty() {
            return Ok(ImageSource::Url(url.to_string()));
        }
    }

    Err("SiliconFlow: missing image URL or b64_json in response".to_string())
}

fn decode_local_sd_response(response: &Value) -> Result<Vec<u8>, String> {
    let image = first_image(response, "Local SD")?
        .as_str()
        .ok_or_else(|| "Local SD: invalid image field in response".to_string())?;
    decode_image_data(image, "Local SD")
}

fn build_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("HTTP client initialization failed: {error}"))
}

fn validate_provider(config: &ProviderConfig) -> Result<(), String> {
    match config.provider.trim() {
        "siliconflow" => {
            if config
                .api_key
                .as_deref()
                .map(str::trim)
                .filter(|key| !key.is_empty())
                .is_none()
            {
                return Err("SiliconFlow API key is required".to_string());
            }
        }
        "localsd" | "local_sd" => {
            if config.local_sd_url.trim().is_empty() {
                return Err("Local SD URL is required".to_string());
            }
        }
        provider => return Err(format!("unsupported image provider: {provider}")),
    }

    Ok(())
}

async fn read_bounded_image_body(mut response: reqwest::Response) -> Result<Vec<u8>, String> {
    validate_content_length(response.content_length())?;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| {
        bounded_sanitized_error(
            "SiliconFlow image download failed",
            &error.to_string(),
            None,
        )
    })? {
        append_bounded_image_chunk(&mut body, &chunk)?;
    }
    finish_image_body(body)
}

async fn read_bounded_error_detail(mut response: reqwest::Response) -> String {
    let mut body = Vec::new();
    let mut truncated = false;
    while let Ok(Some(chunk)) = response.chunk().await {
        let remaining = MAX_ERROR_DETAIL_BYTES.saturating_sub(body.len());
        if remaining == 0 {
            truncated = true;
            break;
        }
        if chunk.len() > remaining {
            body.extend_from_slice(&chunk[..remaining]);
            truncated = true;
            break;
        }
        body.extend_from_slice(&chunk);
    }

    let mut detail = String::from_utf8_lossy(&body).into_owned();
    if truncated {
        detail.push_str(" [truncated]");
    }
    sanitize_provider_error_detail(&detail)
}

async fn parse_json_response(
    response: reqwest::Response,
    provider: &str,
    secret: Option<&str>,
) -> Result<Value, String> {
    let status = response.status();
    if !status.is_success() {
        let detail = read_bounded_error_detail(response).await;
        return Err(bounded_sanitized_error(
            &format!("{provider} API error: {status}"),
            &detail,
            secret,
        ));
    }

    response.json::<Value>().await.map_err(|error| {
        bounded_sanitized_error(
            &format!("{provider} response JSON decode failed"),
            &error.to_string(),
            secret,
        )
    })
}

async fn download_image(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let url = validate_image_download_url(url)?;

    let response = client.get(url).send().await.map_err(|error| {
        bounded_sanitized_error(
            "SiliconFlow image download failed",
            &error.to_string(),
            None,
        )
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("SiliconFlow image download failed: {status}"));
    }

    read_bounded_image_body(response).await
}

async fn generate_siliconflow(config: &ProviderConfig, body: Value) -> Result<Vec<u8>, String> {
    let api_key = config
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .ok_or_else(|| "SiliconFlow API key is required".to_string())?;
    let client = build_http_client()?;
    let response = client
        .post(SILICONFLOW_ENDPOINT)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            bounded_sanitized_error(
                "SiliconFlow request failed",
                &error.to_string(),
                Some(api_key),
            )
        })?;
    let response = parse_json_response(response, "SiliconFlow", Some(api_key)).await?;

    match siliconflow_image_source(&response)? {
        ImageSource::Bytes(bytes) => Ok(bytes),
        ImageSource::Url(url) => download_image(&client, &url).await,
    }
}

async fn generate_local_sd(endpoint: String, body: Value) -> Result<Vec<u8>, String> {
    let client = build_http_client()?;
    let response = client
        .post(endpoint)
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            bounded_sanitized_error("Local SD connection failed", &error.to_string(), None)
        })?;
    let response = parse_json_response(response, "Local SD", None).await?;
    decode_local_sd_response(&response)
}

async fn run_with_operation_deadline<F>(operation: F) -> Result<Vec<u8>, String>
where
    F: std::future::Future<Output = Result<Vec<u8>, String>>,
{
    tokio::time::timeout(OPERATION_TIMEOUT, operation)
        .await
        .map_err(|_| "image generation operation timed out after 120 seconds".to_string())?
}

pub async fn generate_base(config: &ProviderConfig, prompt: &str) -> Result<Vec<u8>, String> {
    validate_provider(config)?;

    run_with_operation_deadline(async {
        match config.provider.trim() {
            "siliconflow" => {
                let body = siliconflow_base_body(&config.base_model, prompt, 256, 256);
                generate_siliconflow(config, body).await
            }
            "localsd" | "local_sd" => {
                let body = local_sd_base_body(prompt, 256, 256);
                generate_local_sd(local_sd_endpoint(&config.local_sd_url, "txt2img"), body).await
            }
            provider => Err(format!("unsupported image provider: {provider}")),
        }
    })
    .await
}

pub async fn generate_row(
    config: &ProviderConfig,
    prompt: &str,
    base_data_url: &str,
) -> Result<Vec<u8>, String> {
    validate_provider(config)?;
    let base_data_url = validate_canonical_data_url(base_data_url)?;

    run_with_operation_deadline(async {
        match config.provider.trim() {
            "siliconflow" => {
                let body = siliconflow_row_body(
                    &config.reference_model,
                    prompt,
                    &base_data_url,
                    2048,
                    256,
                );
                generate_siliconflow(config, body).await
            }
            "localsd" | "local_sd" => {
                let body =
                    local_sd_row_body(prompt, &base_data_url, 2048, 256, config.denoising_strength);
                generate_local_sd(local_sd_endpoint(&config.local_sd_url, "img2img"), body).await
            }
            provider => Err(format!("unsupported image provider: {provider}")),
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

    const BASE_IMAGE: &str = "data:image/png;base64,BASE";

    #[test]
    fn siliconflow_base_body_has_exact_text_to_image_contract() {
        let body = siliconflow_base_body("Tongyi-MAI/Z-Image-Turbo", "base prompt", 256, 256);

        assert_eq!(
            body,
            json!({
                "model": "Tongyi-MAI/Z-Image-Turbo",
                "prompt": "base prompt",
                "image_size": "256x256",
                "num_inference_steps": 20,
                "num_images": 1,
            })
        );
        assert!(body.get("image").is_none());
    }

    #[test]
    fn siliconflow_row_body_has_exact_reference_image_contract() {
        let body = siliconflow_row_body(
            "Qwen/Qwen-Image-Edit-2509",
            "row prompt",
            BASE_IMAGE,
            2048,
            256,
        );

        assert_eq!(
            body,
            json!({
                "model": "Qwen/Qwen-Image-Edit-2509",
                "prompt": "row prompt",
                "image": BASE_IMAGE,
                "image_size": "2048x256",
                "num_inference_steps": 20,
                "num_images": 1,
            })
        );
    }

    #[test]
    fn local_sd_base_body_has_txt2img_contract() {
        assert_eq!(
            local_sd_base_body("base prompt", 256, 256),
            json!({
                "prompt": "base prompt",
                "negative_prompt": LOCAL_SD_NEGATIVE_PROMPT,
                "steps": 20,
                "width": 256,
                "height": 256,
                "batch_size": 1,
            })
        );
    }

    #[test]
    fn local_sd_row_body_has_img2img_contract_and_bounded_denoising() {
        let body = local_sd_row_body("row prompt", BASE_IMAGE, 2048, 256, 0.55);

        assert_eq!(body["prompt"], "row prompt");
        assert_eq!(body["negative_prompt"], LOCAL_SD_NEGATIVE_PROMPT);
        assert_eq!(body["steps"], 20);
        assert_eq!(body["width"], 2048);
        assert_eq!(body["height"], 256);
        assert_eq!(body["batch_size"], 1);
        assert_eq!(body["init_images"][0], BASE_IMAGE);
        assert_eq!(
            body["denoising_strength"].as_f64().unwrap(),
            f64::from(clamp_denoising_strength(0.55))
        );
        assert_eq!(clamp_denoising_strength(0.1), 0.35);
        assert_eq!(clamp_denoising_strength(0.9), 0.75);
    }

    #[test]
    fn non_finite_denoising_uses_the_documented_default() {
        assert_eq!(clamp_denoising_strength(f32::NAN), 0.55);
        assert_eq!(clamp_denoising_strength(f32::INFINITY), 0.55);
        assert_eq!(clamp_denoising_strength(f32::NEG_INFINITY), 0.55);
    }

    #[test]
    fn local_sd_endpoint_trims_legacy_trailing_slashes() {
        assert_eq!(
            local_sd_endpoint("http://127.0.0.1:7860///", "txt2img"),
            "http://127.0.0.1:7860/sdapi/v1/txt2img"
        );
        assert_eq!(
            local_sd_endpoint("http://127.0.0.1:7860/", "img2img"),
            "http://127.0.0.1:7860/sdapi/v1/img2img"
        );
    }

    #[test]
    fn siliconflow_response_decoder_accepts_b64_json_and_data_urls() {
        let bytes = [137_u8, 80, 78, 71];
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);

        let b64_response = json!({ "images": [{ "b64_json": encoded }] });
        assert_eq!(
            siliconflow_image_source(&b64_response).unwrap(),
            ImageSource::Bytes(bytes.to_vec())
        );

        let data_url_response =
            json!({ "images": [{ "b64_json": format!("data:image/png;base64,{encoded}") }] });
        assert_eq!(
            siliconflow_image_source(&data_url_response).unwrap(),
            ImageSource::Bytes(bytes.to_vec())
        );
    }

    #[test]
    fn siliconflow_response_decoder_preserves_download_url() {
        let response = json!({ "images": [{ "url": "https://cdn.example/base.png" }] });

        assert_eq!(
            siliconflow_image_source(&response).unwrap(),
            ImageSource::Url("https://cdn.example/base.png".to_string())
        );
    }

    #[test]
    fn local_sd_response_decoder_accepts_raw_base64_and_data_urls() {
        let bytes = [1_u8, 2, 3, 4];
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);

        let raw_response = json!({ "images": [encoded] });
        assert_eq!(decode_local_sd_response(&raw_response).unwrap(), bytes);

        let data_url_response = json!({ "images": [format!("data:image/jpeg;base64,{encoded}")] });
        assert_eq!(decode_local_sd_response(&data_url_response).unwrap(), bytes);
    }

    #[test]
    fn response_decoders_report_missing_or_invalid_image_fields() {
        let missing = json!({ "images": [] });
        let siliconflow_error = siliconflow_image_source(&missing).unwrap_err();
        assert!(siliconflow_error.contains("missing image"));
        let local_sd_error = decode_local_sd_response(&missing).unwrap_err();
        assert!(local_sd_error.contains("missing image"));

        let invalid = json!({ "images": [{ "b64_json": "not-base64" }] });
        let error = siliconflow_image_source(&invalid).unwrap_err();
        assert!(error.contains("base64"));
    }

    #[test]
    fn unsupported_provider_and_missing_api_key_fail_before_network() {
        let unsupported = ProviderConfig {
            provider: "pollinations".to_string(),
            api_key: Some("unused".to_string()),
            base_model: "base".to_string(),
            reference_model: "reference".to_string(),
            local_sd_url: String::new(),
            denoising_strength: 0.55,
        };
        assert_eq!(
            poll_error(generate_base(&unsupported, "prompt")),
            "unsupported image provider: pollinations"
        );

        let missing_key = ProviderConfig {
            provider: "siliconflow".to_string(),
            api_key: None,
            base_model: "base".to_string(),
            reference_model: "reference".to_string(),
            local_sd_url: String::new(),
            denoising_strength: 0.55,
        };
        assert_eq!(
            poll_error(generate_row(&missing_key, "prompt", BASE_IMAGE)),
            "SiliconFlow API key is required"
        );
    }

    #[test]
    fn image_download_url_validation_allows_https_cdn_and_rejects_unsafe_targets() {
        let valid = validate_image_download_url("https://cdn.example/image.png").unwrap();
        assert_eq!(valid.scheme(), "https");

        for invalid in [
            "",
            "http://cdn.example/image.png",
            "https:///image.png",
            "https://localhost/image.png",
            "https://localhost.localdomain/image.png",
            "https://127.0.0.1/image.png",
            "https://10.0.0.8/image.png",
            "https://192.168.1.8/image.png",
            "https://169.254.1.8/image.png",
            "https://[::1]/image.png",
            "https://[fc00::1]/image.png",
            "https://[fe80::1]/image.png",
            "https://%zz/image.png",
        ] {
            assert!(
                validate_image_download_url(invalid).is_err(),
                "expected URL to be rejected: {invalid}"
            );
        }
    }

    #[test]
    fn canonical_row_data_url_requires_image_base64_metadata_and_payload() {
        let encoded = base64::engine::general_purpose::STANDARD.encode([1_u8, 2, 3]);
        let valid = format!("data:image/png;base64,{encoded}");
        assert_eq!(validate_canonical_data_url(&valid).unwrap(), valid);

        for invalid in [
            "",
            "https://cdn.example/base.png",
            "data:image/png;base64",
            "data:image/png;base64,",
            "data:text/plain;base64,SGVsbG8=",
            "data:image/png,SGVsbG8=",
            "data:image/png;utf8;base64,SGVsbG8=",
            "data:image/;base64,SGVsbG8=",
            "data:image/png;base64,not-base64",
        ] {
            assert!(
                validate_canonical_data_url(invalid).is_err(),
                "expected data URL to be rejected: {invalid}"
            );
        }
    }

    #[test]
    fn image_body_limits_reject_oversized_content_and_empty_body() {
        assert!(validate_content_length(Some(MAX_IMAGE_BYTES as u64)).is_ok());
        assert!(validate_content_length(Some(MAX_IMAGE_BYTES as u64 + 1)).is_err());
        assert!(finish_image_body(Vec::new()).is_err());

        let mut body = Vec::new();
        append_bounded_image_chunk(&mut body, &[1, 2, 3]).unwrap();
        assert_eq!(finish_image_body(body).unwrap(), vec![1, 2, 3]);

        let mut full_body = vec![0_u8; MAX_IMAGE_BYTES];
        assert!(append_bounded_image_chunk(&mut full_body, &[1]).is_err());
    }

    #[test]
    fn provider_error_details_are_url_redacted_and_byte_bounded() {
        let detail = format!(
            "provider failed: https://cdn.example/image.png?X-Amz-Signature={}",
            "secret".repeat(2000)
        );
        let sanitized = sanitize_provider_error_detail(&detail);

        assert!(sanitized.len() <= MAX_ERROR_DETAIL_BYTES);
        assert!(!sanitized.contains("https://"));
        assert!(!sanitized.contains("X-Amz-Signature"));
        assert!(!sanitized.contains("secret"));
    }

    #[test]
    fn network_error_messages_keep_context_without_urls_or_secrets() {
        let detail = "status=503 https://cdn.example/image.png?signature=signed-value api-key";
        let message =
            bounded_sanitized_error("SiliconFlow image download failed", detail, Some("api-key"));

        assert!(message.starts_with("SiliconFlow image download failed:"));
        assert!(message.contains("status=503"));
        assert!(!message.contains("https://"));
        assert!(!message.contains("signed-value"));
        assert!(!message.contains("api-key"));
        assert!(message.len() <= MAX_ERROR_DETAIL_BYTES);
    }

    #[test]
    fn provider_operation_deadline_is_finite_and_shared() {
        assert_eq!(OPERATION_TIMEOUT, Duration::from_secs(120));
    }

    fn poll_error<F>(future: F) -> String
    where
        F: Future<Output = Result<Vec<u8>, String>>,
    {
        let waker = unsafe { Waker::from_raw(noop_raw_waker()) };
        let mut context = Context::from_waker(&waker);
        let mut future = Box::pin(future);

        match Pin::new(&mut future).poll(&mut context) {
            Poll::Ready(Err(error)) => error,
            Poll::Ready(Ok(_)) => panic!("expected provider validation error"),
            Poll::Pending => panic!("provider validation unexpectedly performed network I/O"),
        }
    }

    fn noop_raw_waker() -> RawWaker {
        unsafe fn clone(_: *const ()) -> RawWaker {
            noop_raw_waker()
        }
        unsafe fn wake(_: *const ()) {}
        unsafe fn wake_by_ref(_: *const ()) {}
        unsafe fn drop(_: *const ()) {}

        RawWaker::new(
            std::ptr::null(),
            &RawWakerVTable::new(clone, wake, wake_by_ref, drop),
        )
    }
}
