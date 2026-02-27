#[derive(serde::Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub invite_code: String,
}

#[derive(serde::Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(serde::Deserialize)]
pub struct ExistingUserRow {
    pub id: i32,
}

#[derive(serde::Deserialize)]
pub struct UserAuthRow {
    pub id: i32,
    pub username: String,
    pub password_hash: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct RateLimitState {
    pub count: u32,
    pub window_start_sec: i64,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SessionData {
    pub user_id: i32,
    pub username: String,
    pub issued_at: String,
}

#[derive(serde::Serialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

#[derive(serde::Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    pub token: Option<String>,
    pub username: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct GeneratePrescriptionRequest {
    pub diag_text: String,
}

#[derive(serde::Serialize)]
pub struct GeneratePrescriptionResponse {
    pub success: bool,
    pub result: Option<String>,
    pub message: Option<String>,
}

#[derive(serde::Serialize)]
pub struct RandomPayloadResponse {
    pub success: bool,
    pub med_code: String,
    pub prescription_no: String,
    pub date_input: String,
    pub outpatient_no: String,
    pub diag_date1: String,
    pub diag_date2: String,
    pub a5_bar_widths: Vec<u8>,
    pub a5_bar_margins: Vec<u8>,
    pub diag_bar_widths: Vec<u8>,
    pub diag_bar_margins: Vec<u8>,
}

#[derive(serde::Deserialize)]
pub struct SaveWorkspaceRequest {
    pub title: Option<String>,
    pub payload: serde_json::Value,
}

#[derive(serde::Deserialize)]
pub struct LatestDocRow {
    pub title: String,
    pub payload: String,
    pub updated_at: String,
}

#[derive(serde::Serialize)]
pub struct WorkspaceResponse {
    pub success: bool,
    pub title: Option<String>,
    pub payload: Option<serde_json::Value>,
    pub updated_at: Option<String>,
    pub message: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct PdfGenerateRequest {
    pub title: Option<String>,
    pub diagnosis: Option<String>,
    pub prescription: Option<String>,
    pub note: Option<String>,
    pub image_data_url: Option<String>,
    pub page_mode: Option<String>,
}
