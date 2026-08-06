use crate::commands::generation::types::ProviderConfig;
use base64::Engine as _;
use serde_json::Value;
use std::time::Duration;

const SILICONFLOW_ENDPOINT: &str = "https://api.siliconflow.cn/v1/images/generations";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
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
    let encoded = if value.starts_with("data:") {
        value
            .split_once(',')
            .map(|(_, payload)| payload.trim())
            .ok_or_else(|| format!("{provider}: invalid data URL image"))?
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

async fn parse_json_response(response: reqwest::Response, provider: &str) -> Result<Value, String> {
    let status = response.status();
    if !status.is_success() {
        return Err(format!("{provider} API error: {status}"));
    }

    response
        .json::<Value>()
        .await
        .map_err(|error| format!("{provider} response JSON decode failed: {error}"))
}

async fn download_image(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    if url.trim_start().starts_with("data:") {
        return decode_image_data(url, "SiliconFlow");
    }

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("SiliconFlow image download failed: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("SiliconFlow image download failed: {status}"));
    }

    response
        .bytes()
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(|error| format!("SiliconFlow image download failed: {error}"))
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
        .map_err(|error| format!("SiliconFlow request failed: {error}"))?;
    let response = parse_json_response(response, "SiliconFlow").await?;

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
        .map_err(|error| format!("Local SD connection failed: {error}"))?;
    let response = parse_json_response(response, "Local SD").await?;
    decode_local_sd_response(&response)
}

pub async fn generate_base(config: &ProviderConfig, prompt: &str) -> Result<Vec<u8>, String> {
    validate_provider(config)?;

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
}

pub async fn generate_row(
    config: &ProviderConfig,
    prompt: &str,
    base_data_url: &str,
) -> Result<Vec<u8>, String> {
    validate_provider(config)?;
    if base_data_url.trim().is_empty() {
        return Err("canonical Base image is required".to_string());
    }

    match config.provider.trim() {
        "siliconflow" => {
            let body =
                siliconflow_row_body(&config.reference_model, prompt, base_data_url, 2048, 256);
            generate_siliconflow(config, body).await
        }
        "localsd" | "local_sd" => {
            let body =
                local_sd_row_body(prompt, base_data_url, 2048, 256, config.denoising_strength);
            generate_local_sd(local_sd_endpoint(&config.local_sd_url, "img2img"), body).await
        }
        provider => Err(format!("unsupported image provider: {provider}")),
    }
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
