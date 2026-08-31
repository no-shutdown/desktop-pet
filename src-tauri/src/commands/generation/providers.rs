use crate::commands::generation::types::ProviderConfig;
use base64::Engine as _;
use serde_json::Value;
use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
    time::Duration,
};

const SILICONFLOW_ENDPOINT: &str = "https://api.siliconflow.cn/v1/images/generations";
const WANXIANG_T2I_ENDPOINT: &str =
    "https://dashscope.aliyuncs.com/api/v1/services/aigc/text2image/image-synthesis";
const WANXIANG_EDIT_ENDPOINT: &str =
    "https://dashscope.aliyuncs.com/api/v1/services/aigc/image2image/image-synthesis";
/// Newer wan2.6 / wan2.7 models share a single async image generation endpoint
/// that accepts both text-to-image and reference-based edit requests.
const WAN_IMAGE_GENERATION_ENDPOINT: &str =
    "https://dashscope.aliyuncs.com/api/v1/services/aigc/image-generation/generation";
const WANXIANG_TASK_ENDPOINT_PREFIX: &str = "https://dashscope.aliyuncs.com/api/v1/tasks/";
const WANXIANG_POLL_INTERVAL: Duration = Duration::from_secs(3);
/// Wanxiang t2i-turbo supports a fixed set of square/rectangular sizes. 1024*1024 is
/// the closest square that always works; the caller downsamples it back to the
/// canonical 256×256 base via normalize_base_image.
const WANXIANG_BASE_SIZE: (u32, u32) = (1024, 1024);
const OPERATION_TIMEOUT: Duration = Duration::from_secs(180);
const REQUEST_TIMEOUT: Duration = OPERATION_TIMEOUT;
const MAX_IMAGE_BYTES: usize = 16 * 1024 * 1024;
const MAX_ENCODED_IMAGE_BYTES: usize = ((MAX_IMAGE_BYTES + 2) / 3) * 4;
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

pub fn siliconflow_base_body_with_references(
    model: &str,
    prompt: &str,
    character_image_data_url: &str,
    style_image_data_url: &str,
) -> Result<Value, String> {
    if model.trim() != "Qwen/Qwen-Image-Edit-2509" {
        return Err("SiliconFlow style references require Qwen/Qwen-Image-Edit-2509".to_string());
    }

    Ok(serde_json::json!({
        "model": model,
        "prompt": prompt,
        "image": character_image_data_url,
        "image2": style_image_data_url,
        "num_inference_steps": 20,
    }))
}

pub fn siliconflow_row_body(
    model: &str,
    prompt: &str,
    image_data_url: &str,
    width: u32,
    height: u32,
) -> Value {
    let mut body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "image": image_data_url,
        "num_inference_steps": 20,
    })
    ;

    if !matches!(
        model.trim(),
        "Qwen/Qwen-Image-Edit-2509" | "Qwen/Qwen-Image-Edit"
    ) {
        body["image_size"] = serde_json::json!(format!("{width}x{height}"));
        body["num_images"] = serde_json::json!(1);
    }

    body
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

/// Legacy `wanx*` models (wanx-v1, wanx2.1-t2i-*, wanx2.1-imageedit) use the
/// old `/text2image` and `/image2image` endpoints with an `input.prompt`
/// body. Newer `wan2.6-*` / `wan2.7-*` models use the unified
/// `/image-generation/generation` endpoint with a chat-style messages body.
pub fn is_new_wan_model(model: &str) -> bool {
    let model = model.trim();
    model.starts_with("wan2.") && !model.starts_with("wanx")
}

pub fn wanxiang_endpoint_for_base(model: &str) -> &'static str {
    if is_new_wan_model(model) {
        WAN_IMAGE_GENERATION_ENDPOINT
    } else {
        WANXIANG_T2I_ENDPOINT
    }
}

pub fn wanxiang_endpoint_for_row(model: &str) -> &'static str {
    if is_new_wan_model(model) {
        WAN_IMAGE_GENERATION_ENDPOINT
    } else {
        WANXIANG_EDIT_ENDPOINT
    }
}

pub fn wanxiang_base_body(model: &str, prompt: &str, width: u32, height: u32) -> Value {
    if is_new_wan_model(model) {
        // wan2.6 / wan2.7 chat-style body. `size` accepts shortcuts like "1K"/"2K"
        // or explicit "WxH" (asterisk); the caller downscales the result anyway.
        serde_json::json!({
            "model": model,
            "input": {
                "messages": [{
                    "role": "user",
                    "content": [{"text": prompt}],
                }],
            },
            "parameters": {
                "size": format!("{width}*{height}"),
                "n": 1,
            },
        })
    } else {
        serde_json::json!({
            "model": model,
            "input": {
                "prompt": prompt,
            },
            "parameters": {
                "size": format!("{width}*{height}"),
                "n": 1,
            },
        })
    }
}

pub fn wanxiang_base_body_with_references(
    model: &str,
    prompt: &str,
    character_image_data_url: &str,
    style_image_data_url: &str,
) -> Result<Value, String> {
    if !matches!(
        model.trim(),
        "wan2.6-image" | "wan2.7-image" | "wan2.7-image-pro"
    ) {
        return Err("Wanxiang style references require wan2.6 or wan2.7 models".to_string());
    }

    Ok(serde_json::json!({
        "model": model,
        "input": {
            "messages": [{
                "role": "user",
                "content": [
                    {"image": character_image_data_url},
                    {"image": style_image_data_url},
                    {"text": prompt},
                ],
            }],
        },
        "parameters": { "size": "1024*1024", "n": 1 },
    }))
}

pub fn wanxiang_row_body(model: &str, prompt: &str, base_image_data_url: &str) -> Value {
    if is_new_wan_model(model) {
        // wan2.* takes reference images inline in the chat content array.
        // Output aspect follows the reference image dimensions.
        serde_json::json!({
            "model": model,
            "input": {
                "messages": [{
                    "role": "user",
                    "content": [
                        {"image": base_image_data_url},
                        {"text": prompt},
                    ],
                }],
            },
            "parameters": {
                "n": 1,
            },
        })
    } else {
        serde_json::json!({
            "model": model,
            "input": {
                "prompt": prompt,
                "function": "description_edit",
                "base_image_url": base_image_data_url,
            },
            "parameters": {
                "n": 1,
            },
        })
    }
}

fn is_valid_image_subtype(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '.' | '-')
        })
}

fn validate_encoded_payload_length(payload: &str, context: &str) -> Result<(), String> {
    if payload.len() > MAX_ENCODED_IMAGE_BYTES {
        return Err(format!(
            "{context} encoded payload exceeds the {} MiB limit",
            MAX_IMAGE_BYTES / (1024 * 1024)
        ));
    }
    Ok(())
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
    validate_encoded_payload_length(payload, "canonical Base image")?;
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
    if decoded.len() > MAX_IMAGE_BYTES {
        return Err(format!(
            "canonical Base image exceeds the {} MiB limit",
            MAX_IMAGE_BYTES / (1024 * 1024)
        ));
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

fn validate_resolved_addresses(addresses: &[SocketAddr]) -> Result<SocketAddr, String> {
    let chosen = addresses
        .first()
        .copied()
        .ok_or_else(|| "SiliconFlow image URL resolved to no addresses".to_string())?;
    if addresses.iter().any(|address| is_blocked_ip(address.ip())) {
        return Err("SiliconFlow image URL resolved to a private or local address".to_string());
    }
    Ok(chosen)
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

struct ResolvedImageUrl {
    url: reqwest::Url,
    host: String,
    address: SocketAddr,
}

async fn resolve_image_download_url(value: &str) -> Result<ResolvedImageUrl, String> {
    let url = validate_image_download_url(value)?;
    let host = url
        .host_str()
        .filter(|host| !host.trim().is_empty())
        .ok_or_else(|| "SiliconFlow image URL must have a non-empty host".to_string())?;
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host)
        .to_string();
    let port = url.port().unwrap_or(443);
    let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|error| {
            bounded_sanitized_error(
                "SiliconFlow image URL DNS lookup failed",
                &error.to_string(),
                None,
            )
        })?
        .collect();
    let address = validate_resolved_addresses(&addresses)?;

    Ok(ResolvedImageUrl { url, host, address })
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
    validate_encoded_payload_length(encoded, provider)?;

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| format!("{provider} base64 decode error: {error}"))?;
    if bytes.is_empty() {
        return Err(format!("{provider}: decoded image is empty"));
    }
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(format!(
            "{provider}: decoded image exceeds the {} MiB limit",
            MAX_IMAGE_BYTES / (1024 * 1024)
        ));
    }
    Ok(bytes)
}

fn validate_content_length_for(content_length: Option<u64>, context: &str) -> Result<(), String> {
    match content_length {
        Some(0) => Err(format!("{context} body is empty")),
        Some(length) if length > MAX_IMAGE_BYTES as u64 => Err(format!(
            "{context} exceeds the {} MiB limit",
            MAX_IMAGE_BYTES / (1024 * 1024)
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
fn validate_content_length(content_length: Option<u64>) -> Result<(), String> {
    validate_content_length_for(content_length, "SiliconFlow image response")
}

fn append_bounded_chunk(body: &mut Vec<u8>, chunk: &[u8], context: &str) -> Result<(), String> {
    let next_length = body
        .len()
        .checked_add(chunk.len())
        .ok_or_else(|| format!("{context} is too large"))?;
    if next_length > MAX_IMAGE_BYTES {
        return Err(format!(
            "{context} exceeds the {} MiB limit",
            MAX_IMAGE_BYTES / (1024 * 1024)
        ));
    }
    body.extend_from_slice(chunk);
    Ok(())
}

#[cfg(test)]
fn append_bounded_image_chunk(body: &mut Vec<u8>, chunk: &[u8]) -> Result<(), String> {
    append_bounded_chunk(body, chunk, "SiliconFlow image response")
}

fn finish_bounded_body(body: Vec<u8>, context: &str) -> Result<Vec<u8>, String> {
    if body.is_empty() {
        return Err(format!("{context} body is empty"));
    }
    Ok(body)
}

#[cfg(test)]
fn finish_image_body(body: Vec<u8>) -> Result<Vec<u8>, String> {
    finish_bounded_body(body, "SiliconFlow image response")
}

fn parse_bounded_json_body(body: &[u8], provider: &str) -> Result<Value, String> {
    if body.is_empty() {
        return Err(format!("{provider} response body is empty"));
    }
    if body.len() > MAX_IMAGE_BYTES {
        return Err(format!(
            "{provider} response exceeds the {} MiB limit",
            MAX_IMAGE_BYTES / (1024 * 1024)
        ));
    }
    serde_json::from_slice(body).map_err(|error| {
        bounded_sanitized_error(
            &format!("{provider} response JSON is invalid"),
            &error.to_string(),
            None,
        )
    })
}

fn next_http_url_start(value: &str) -> Option<usize> {
    let lower = value.to_ascii_lowercase();
    let http = lower.find("http://");
    let https = lower.find("https://");
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

fn client_initialization_error(context: &str, detail: &str, secret: Option<&str>) -> String {
    bounded_sanitized_error(
        &format!("{context} HTTP client initialization failed"),
        detail,
        secret,
    )
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

fn build_http_client(provider: &str, secret: Option<&str>) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| client_initialization_error(provider, &error.to_string(), secret))
}

fn build_resolved_http_client(host: &str, address: SocketAddr) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .resolve(host, address)
        .build()
        .map_err(|error| {
            client_initialization_error("SiliconFlow image download", &error.to_string(), None)
        })
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
        "wanxiang" => {
            if config
                .api_key
                .as_deref()
                .map(str::trim)
                .filter(|key| !key.is_empty())
                .is_none()
            {
                return Err("Wanxiang API key is required".to_string());
            }
        }
        "localsd" | "local_sd" => {
            if config.local_sd_url.trim().is_empty() {
                return Err("Local SD URL is required".to_string());
            }
        }
        provider => {
            return Err(bounded_sanitized_error(
                "unsupported image provider",
                provider,
                None,
            ))
        }
    }

    Ok(())
}

async fn read_bounded_response_body(
    mut response: reqwest::Response,
    context: &str,
) -> Result<Vec<u8>, String> {
    validate_content_length_for(response.content_length(), context)?;
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| bounded_sanitized_error(context, &error.to_string(), None))?
    {
        append_bounded_chunk(&mut body, &chunk, context)?;
    }
    finish_bounded_body(body, context)
}

async fn read_bounded_image_body(response: reqwest::Response) -> Result<Vec<u8>, String> {
    read_bounded_response_body(response, "SiliconFlow image response").await
}

async fn read_bounded_error_detail(mut response: reqwest::Response) -> String {
    let mut body = Vec::new();
    let mut truncated = false;
    while body.len() < MAX_ERROR_DETAIL_BYTES {
        let Ok(Some(chunk)) = response.chunk().await else {
            break;
        };
        let remaining = MAX_ERROR_DETAIL_BYTES - body.len();
        if chunk.len() > remaining {
            body.extend_from_slice(&chunk[..remaining]);
            truncated = true;
            break;
        }
        body.extend_from_slice(&chunk);
        if body.len() == MAX_ERROR_DETAIL_BYTES {
            truncated = true;
            break;
        }
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

    let body = read_bounded_response_body(response, &format!("{provider} response")).await?;
    parse_bounded_json_body(&body, provider).map_err(|error| {
        bounded_sanitized_error(
            &format!("{provider} response JSON decode failed"),
            &error,
            secret,
        )
    })
}

async fn download_image(url: &str) -> Result<Vec<u8>, String> {
    let resolved = resolve_image_download_url(url).await?;
    let client = build_resolved_http_client(&resolved.host, resolved.address)?;

    let response = client.get(resolved.url).send().await.map_err(|error| {
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
    let client = build_http_client("SiliconFlow", Some(api_key))?;
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
        ImageSource::Url(url) => download_image(&url).await,
    }
}

/// Wan async responses put the image URL in one of two places depending on the
/// model family:
///   * legacy `wanx*`  → `output.results[0].url`
///   * new     `wan2.*` → `output.choices[0].message.content[0].image`
/// Try both so a single poller handles either.
fn extract_wanxiang_image_url(payload: &Value) -> Option<String> {
    if let Some(url) = payload
        .pointer("/output/results/0/url")
        .and_then(Value::as_str)
    {
        return Some(url.to_string());
    }
    let content = payload
        .pointer("/output/choices/0/message/content")
        .and_then(Value::as_array)?;
    for item in content {
        if let Some(url) = item.get("image").and_then(Value::as_str) {
            return Some(url.to_string());
        }
        if let Some(url) = item.get("url").and_then(Value::as_str) {
            return Some(url.to_string());
        }
    }
    None
}

async fn submit_wanxiang_task(
    api_key: &str,
    endpoint: &str,
    body: Value,
) -> Result<String, String> {
    let client = build_http_client("Wanxiang", Some(api_key))?;
    let response = client
        .post(endpoint)
        .bearer_auth(api_key)
        .header("X-DashScope-Async", "enable")
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            bounded_sanitized_error(
                "Wanxiang request failed",
                &error.to_string(),
                Some(api_key),
            )
        })?;
    let payload = parse_json_response(response, "Wanxiang", Some(api_key)).await?;
    payload
        .pointer("/output/task_id")
        .and_then(Value::as_str)
        .map(|task_id| task_id.to_string())
        .ok_or_else(|| "Wanxiang: submit response missing task_id".to_string())
}

fn parse_wanxiang_task_id(value: &str) -> Result<&str, String> {
    if value.is_empty() {
        return Err("Wanxiang: task id is empty".to_string());
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err("Wanxiang: task id contains an invalid character".to_string());
    }
    Ok(value)
}

async fn poll_wanxiang_task(api_key: &str, task_id: &str) -> Result<String, String> {
    let safe_task_id = parse_wanxiang_task_id(task_id)?;
    let client = build_http_client("Wanxiang", Some(api_key))?;
    let url = format!("{WANXIANG_TASK_ENDPOINT_PREFIX}{safe_task_id}");
    loop {
        let response = client
            .get(&url)
            .bearer_auth(api_key)
            .send()
            .await
            .map_err(|error| {
                bounded_sanitized_error(
                    "Wanxiang task poll failed",
                    &error.to_string(),
                    Some(api_key),
                )
            })?;
        let payload = parse_json_response(response, "Wanxiang", Some(api_key)).await?;
        let status = payload
            .pointer("/output/task_status")
            .and_then(Value::as_str)
            .unwrap_or("");
        match status {
            "SUCCEEDED" => {
                let image_url = extract_wanxiang_image_url(&payload).ok_or_else(|| {
                    "Wanxiang: task succeeded but response is missing an image URL".to_string()
                })?;
                return Ok(image_url);
            }
            "FAILED" | "CANCELED" | "UNKNOWN" => {
                let detail = payload
                    .pointer("/output/message")
                    .and_then(Value::as_str)
                    .or_else(|| payload.pointer("/output/code").and_then(Value::as_str))
                    .unwrap_or(status);
                return Err(bounded_sanitized_error(
                    &format!("Wanxiang task {status}"),
                    detail,
                    Some(api_key),
                ));
            }
            _ => {
                tokio::time::sleep(WANXIANG_POLL_INTERVAL).await;
            }
        }
    }
}

async fn generate_wanxiang(
    config: &ProviderConfig,
    endpoint: &str,
    body: Value,
) -> Result<Vec<u8>, String> {
    let api_key = config
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .ok_or_else(|| "Wanxiang API key is required".to_string())?;
    let task_id = submit_wanxiang_task(api_key, endpoint, body).await?;
    let image_url = poll_wanxiang_task(api_key, &task_id).await?;
    download_image(&image_url).await
}

async fn generate_local_sd(endpoint: String, body: Value) -> Result<Vec<u8>, String> {
    let client = build_http_client("Local SD", None)?;
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
            "wanxiang" => {
                let endpoint = wanxiang_endpoint_for_base(&config.base_model);
                let body = wanxiang_base_body(
                    &config.base_model,
                    prompt,
                    WANXIANG_BASE_SIZE.0,
                    WANXIANG_BASE_SIZE.1,
                );
                generate_wanxiang(config, endpoint, body).await
            }
            "localsd" | "local_sd" => {
                let body = local_sd_base_body(prompt, 256, 256);
                generate_local_sd(local_sd_endpoint(&config.local_sd_url, "txt2img"), body).await
            }
            provider => Err(bounded_sanitized_error(
                "unsupported image provider",
                provider,
                None,
            )),
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
            "wanxiang" => {
                let endpoint = wanxiang_endpoint_for_row(&config.reference_model);
                let body = wanxiang_row_body(&config.reference_model, prompt, &base_data_url);
                generate_wanxiang(config, endpoint, body).await
            }
            "localsd" | "local_sd" => {
                let body =
                    local_sd_row_body(prompt, &base_data_url, 2048, 256, config.denoising_strength);
                generate_local_sd(local_sd_endpoint(&config.local_sd_url, "img2img"), body).await
            }
            provider => Err(bounded_sanitized_error(
                "unsupported image provider",
                provider,
                None,
            )),
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::future::Future;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
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
    fn siliconflow_base_body_with_references_has_exact_two_image_contract() {
        let body = siliconflow_base_body_with_references(
            "Qwen/Qwen-Image-Edit-2509",
            "base prompt",
            "data:image/jpeg;base64,CHARACTER",
            "data:image/png;base64,STYLE",
        )
        .unwrap();

        assert_eq!(
            body,
            json!({
                "model": "Qwen/Qwen-Image-Edit-2509",
                "prompt": "base prompt",
                "image": "data:image/jpeg;base64,CHARACTER",
                "image2": "data:image/png;base64,STYLE",
                "num_inference_steps": 20,
            })
        );
        assert_eq!(body["image"], "data:image/jpeg;base64,CHARACTER");
        assert_eq!(body["image2"], "data:image/png;base64,STYLE");
        assert_eq!(body["prompt"], "base prompt");
        assert!(body.get("image_size").is_none());
    }

    #[test]
    fn siliconflow_base_body_with_references_rejects_kolors() {
        let error = siliconflow_base_body_with_references(
            "Kwai-Kolors/Kolors",
            "base prompt",
            "data:image/jpeg;base64,CHARACTER",
            "data:image/png;base64,STYLE",
        )
        .unwrap_err();

        assert!(error.contains("Qwen/Qwen-Image-Edit-2509"));
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
                "num_inference_steps": 20,
            })
        );
    }

    #[test]
    fn wanxiang_base_body_uses_legacy_prompt_shape_for_wanx_models() {
        let body = wanxiang_base_body("wanx2.1-t2i-turbo", "base prompt", 1024, 1024);

        assert_eq!(
            body,
            json!({
                "model": "wanx2.1-t2i-turbo",
                "input": { "prompt": "base prompt" },
                "parameters": { "size": "1024*1024", "n": 1 },
            })
        );
    }

    #[test]
    fn wanxiang_base_body_uses_chat_messages_shape_for_wan27_models() {
        let body = wanxiang_base_body("wan2.7-image", "base prompt", 1024, 1024);

        assert_eq!(
            body,
            json!({
                "model": "wan2.7-image",
                "input": {
                    "messages": [{
                        "role": "user",
                        "content": [{"text": "base prompt"}],
                    }],
                },
                "parameters": { "size": "1024*1024", "n": 1 },
            })
        );
    }

    #[test]
    fn wanxiang_base_body_with_references_has_exact_two_image_chat_contract() {
        let body = wanxiang_base_body_with_references(
            "wan2.7-image",
            "base prompt",
            "data:image/jpeg;base64,CHARACTER",
            "data:image/png;base64,STYLE",
        )
        .unwrap();

        assert_eq!(
            body,
            json!({
                "model": "wan2.7-image",
                "input": {
                    "messages": [{
                        "role": "user",
                        "content": [
                            {"image": "data:image/jpeg;base64,CHARACTER"},
                            {"image": "data:image/png;base64,STYLE"},
                            {"text": "base prompt"},
                        ],
                    }],
                },
                "parameters": { "size": "1024*1024", "n": 1 },
            })
        );
        let content = &body["input"]["messages"][0]["content"];
        assert_eq!(content[0]["image"], "data:image/jpeg;base64,CHARACTER");
        assert_eq!(content[1]["image"], "data:image/png;base64,STYLE");
        assert_eq!(content[2]["text"], "base prompt");
    }

    #[test]
    fn wanxiang_base_body_with_references_rejects_legacy_models() {
        let error = wanxiang_base_body_with_references(
            "wanx2.1-t2i-turbo",
            "base prompt",
            "data:image/jpeg;base64,CHARACTER",
            "data:image/png;base64,STYLE",
        )
        .unwrap_err();

        assert!(error.contains("wan2.6") || error.contains("wan2.7"));
    }

    #[test]
    fn wanxiang_base_body_with_references_rejects_unsupported_wan2_models() {
        for model in ["wan2.5-image", "wan2.foo"] {
            let error = wanxiang_base_body_with_references(
                model,
                "base prompt",
                "data:image/jpeg;base64,CHARACTER",
                "data:image/png;base64,STYLE",
            )
            .unwrap_err();

            assert!(
                error.contains("wan2.6") || error.contains("wan2.7"),
                "unexpected error for {model}: {error}"
            );
        }
    }

    #[test]
    fn wanxiang_row_body_uses_legacy_image_edit_shape_for_wanx_models() {
        let body = wanxiang_row_body("wanx2.1-imageedit", "row prompt", BASE_IMAGE);

        assert_eq!(
            body,
            json!({
                "model": "wanx2.1-imageedit",
                "input": {
                    "prompt": "row prompt",
                    "function": "description_edit",
                    "base_image_url": BASE_IMAGE,
                },
                "parameters": { "n": 1 },
            })
        );
    }

    #[test]
    fn wanxiang_row_body_uses_chat_messages_shape_for_wan27_models() {
        let body = wanxiang_row_body("wan2.7-image-pro", "row prompt", BASE_IMAGE);

        assert_eq!(
            body,
            json!({
                "model": "wan2.7-image-pro",
                "input": {
                    "messages": [{
                        "role": "user",
                        "content": [
                            {"image": BASE_IMAGE},
                            {"text": "row prompt"},
                        ],
                    }],
                },
                "parameters": { "n": 1 },
            })
        );
    }

    #[test]
    fn wanxiang_url_extraction_supports_both_legacy_and_wan27_response_shapes() {
        let legacy = json!({
            "output": {
                "task_status": "SUCCEEDED",
                "results": [{"url": "https://cdn.example/legacy.png"}]
            }
        });
        assert_eq!(
            extract_wanxiang_image_url(&legacy),
            Some("https://cdn.example/legacy.png".to_string())
        );

        let wan27 = json!({
            "output": {
                "task_status": "SUCCEEDED",
                "choices": [{
                    "finish_reason": "stop",
                    "message": {
                        "role": "assistant",
                        "content": [
                            {"image": "https://cdn.example/wan27.png", "type": "image"}
                        ]
                    }
                }]
            }
        });
        assert_eq!(
            extract_wanxiang_image_url(&wan27),
            Some("https://cdn.example/wan27.png".to_string())
        );

        let missing = json!({"output": {"task_status": "SUCCEEDED"}});
        assert_eq!(extract_wanxiang_image_url(&missing), None);
    }

    #[test]
    fn wanxiang_endpoint_dispatch_switches_by_model_family() {
        assert_eq!(
            wanxiang_endpoint_for_base("wanx2.1-t2i-turbo"),
            WANXIANG_T2I_ENDPOINT
        );
        assert_eq!(
            wanxiang_endpoint_for_base("wan2.7-image"),
            WAN_IMAGE_GENERATION_ENDPOINT
        );
        assert_eq!(
            wanxiang_endpoint_for_row("wanx2.1-imageedit"),
            WANXIANG_EDIT_ENDPOINT
        );
        assert_eq!(
            wanxiang_endpoint_for_row("wan2.6-image"),
            WAN_IMAGE_GENERATION_ENDPOINT
        );
        assert!(!is_new_wan_model("wanx2.1-t2i-turbo"));
        assert!(is_new_wan_model("wan2.7-image"));
        assert!(is_new_wan_model("wan2.6-image"));
    }

    #[test]
    fn wanxiang_task_ids_reject_path_traversal_and_special_chars() {
        assert!(parse_wanxiang_task_id("").is_err());
        assert!(parse_wanxiang_task_id("../etc/passwd").is_err());
        assert!(parse_wanxiang_task_id("id with spaces").is_err());
        assert!(parse_wanxiang_task_id("id?query").is_err());
        assert_eq!(parse_wanxiang_task_id("abc-DEF_123").unwrap(), "abc-DEF_123");
    }

    #[test]
    fn siliconflow_kolors_row_body_keeps_explicit_output_size() {
        let body = siliconflow_row_body(
            "Kwai-Kolors/Kolors",
            "row prompt",
            BASE_IMAGE,
            2048,
            256,
        );

        assert_eq!(body["image_size"], "2048x256");
        assert_eq!(body["num_images"], 1);
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
    fn wanxiang_missing_api_key_fails_before_network() {
        let missing_key = ProviderConfig {
            provider: "wanxiang".to_string(),
            api_key: None,
            base_model: "wanx2.1-t2i-turbo".to_string(),
            reference_model: "wanx2.1-imageedit".to_string(),
            local_sd_url: String::new(),
            denoising_strength: 0.55,
        };
        assert_eq!(
            poll_error(generate_base(&missing_key, "prompt")),
            "Wanxiang API key is required"
        );
        assert_eq!(
            poll_error(generate_row(&missing_key, "prompt", BASE_IMAGE)),
            "Wanxiang API key is required"
        );
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
            "provider failed: HTTPS://cdn.example/image.png?X-Amz-Signature={}",
            "secret".repeat(2000)
        );
        let sanitized = sanitize_provider_error_detail(&detail);

        assert!(sanitized.len() <= MAX_ERROR_DETAIL_BYTES);
        assert!(!sanitized.contains("HTTPS://"));
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
    fn client_initialization_errors_keep_provider_context_and_are_sanitized() {
        let detail = format!(
            "HTTPS://signed.example/client?token={} api-key",
            "secret".repeat(2000)
        );
        let message = client_initialization_error("SiliconFlow", &detail, Some("api-key"));

        assert!(message.starts_with("SiliconFlow HTTP client initialization failed:"));
        assert!(message.len() <= MAX_ERROR_DETAIL_BYTES);
        assert!(!message.contains("HTTPS://"));
        assert!(!message.contains("token"));
        assert!(!message.contains(&"secret".repeat(200)));
        assert!(!message.contains("api-key"));
    }

    #[test]
    fn successful_json_and_decoded_images_are_size_bounded() {
        assert!(parse_bounded_json_body(b"", "SiliconFlow").is_err());
        assert!(parse_bounded_json_body(&vec![b' '; MAX_IMAGE_BYTES + 1], "SiliconFlow").is_err());
        assert!(parse_bounded_json_body(br#"{"images": []}"#, "SiliconFlow").is_ok());

        let encoded =
            base64::engine::general_purpose::STANDARD.encode(vec![0_u8; MAX_IMAGE_BYTES + 1]);
        assert!(decode_image_data(&encoded, "SiliconFlow").is_err());
        assert!(validate_canonical_data_url(&format!("data:image/png;base64,{encoded}")).is_err());
        assert!(decode_local_sd_response(&json!({ "images": [encoded] })).is_err());
    }

    #[test]
    fn oversized_encoded_image_payload_is_rejected_before_decode() {
        let oversized = "A".repeat(MAX_ENCODED_IMAGE_BYTES + 1);
        let raw_error = decode_image_data(&oversized, "SiliconFlow").unwrap_err();
        assert!(raw_error.contains("encoded payload"));

        let data_url = format!("data:image/png;base64,{oversized}");
        let canonical_error = validate_canonical_data_url(&data_url).unwrap_err();
        assert!(canonical_error.contains("encoded payload"));
    }

    #[test]
    fn resolved_private_addresses_are_rejected_before_download() {
        let safe = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 443);
        assert_eq!(validate_resolved_addresses(&[safe]).unwrap(), safe);

        for unsafe_address in [
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 443),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 443),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1)), 443),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 443),
            SocketAddr::new(Ipv4Addr::BROADCAST.into(), 443),
            SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 443),
            SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 443),
            SocketAddr::new(IpAddr::V6("fc00::1".parse().unwrap()), 443),
        ] {
            assert!(
                validate_resolved_addresses(&[unsafe_address]).is_err(),
                "expected address to be rejected: {unsafe_address}"
            );
        }
    }

    #[test]
    fn unsupported_provider_names_are_bounded_and_sanitized() {
        let provider = format!("HTTPS://signed.example/?token={}", "secret".repeat(2000));
        let config = ProviderConfig {
            provider,
            api_key: None,
            base_model: "base".to_string(),
            reference_model: "reference".to_string(),
            local_sd_url: String::new(),
            denoising_strength: 0.55,
        };

        let error = poll_error(generate_base(&config, "prompt"));
        assert!(error.starts_with("unsupported image provider:"));
        assert!(error.len() <= MAX_ERROR_DETAIL_BYTES);
        assert!(!error.contains("HTTPS://"));
        assert!(!error.contains("secret"));
    }

    #[test]
    fn provider_operation_deadline_is_finite_and_shared() {
        assert_eq!(OPERATION_TIMEOUT, Duration::from_secs(180));
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
