use base64::Engine;
use sha2::{Digest, Sha256};
use worker::js_sys::{Date, Math};
use worker::wasm_bindgen::JsValue;
use worker::Request;

pub fn extract_ip(req: &Request) -> String {
    req.headers()
        .get("CF-Connecting-IP")
        .ok()
        .flatten()
        .or_else(|| req.headers().get("X-Forwarded-For").ok().flatten())
        .and_then(|v| v.split(',').next().map(str::trim).map(str::to_string))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn hash_password(username: &str, password: &str, pepper: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(username.as_bytes());
    hasher.update(b":");
    hasher.update(password.as_bytes());
    hasher.update(b":");
    hasher.update(pepper.as_bytes());
    let out = hasher.finalize();
    hex::encode(out)
}

pub fn generate_session_token(username: &str, pepper: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(username.as_bytes());
    hasher.update(b":");
    hasher.update(now_iso().as_bytes());
    hasher.update(b":");
    hasher.update(Math::random().to_string().as_bytes());
    hasher.update(b":");
    hasher.update(pepper.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn now_iso() -> String {
    Date::new_0()
        .to_iso_string()
        .as_string()
        .unwrap_or_default()
}

pub fn rand_digit() -> i32 {
    rand_range_int(0, 9)
}

pub fn rand_range_int(min: i32, max: i32) -> i32 {
    (Math::random() * ((max - min + 1) as f64)).floor() as i32 + min
}

pub fn decode_data_url_png(input: &str) -> Option<Vec<u8>> {
    let prefix = "data:image/png;base64,";
    let payload = input.strip_prefix(prefix)?;
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .ok()
}

pub fn random_date_from_now() -> Date {
    let now = Date::new_0();
    let offset_days = rand_range_int(0, 89) as f64;
    let random_ts = now.get_time() - (offset_days * 24.0 * 60.0 * 60.0 * 1000.0);
    Date::new(&JsValue::from_f64(random_ts))
}
