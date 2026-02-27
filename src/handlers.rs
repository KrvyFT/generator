use crate::auth::{apply_rate_limit, require_auth};
use crate::constants::{D1_BINDING, DEFAULT_SUPPORT_EMAIL, KV_BINDING};
use crate::models::*;
use crate::response::{json_with_status, with_cors_headers};
use crate::utils::{
    build_image_pdf, build_simple_pdf, decode_data_url_jpeg, generate_session_token, hash_password,
    jpeg_dimensions, now_iso, rand_digit, rand_range_int, random_date_from_now, split_multiline,
};
use worker::wasm_bindgen::JsValue;
use worker::*;

pub async fn register(mut req: Request, env: &Env, cors_origin: &str) -> Result<HttpResponse> {
    if let Err(msg) = apply_rate_limit(&req, env, "register").await {
        return json_with_status(
            429,
            &ApiResponse {
                success: false,
                message: msg,
            },
            cors_origin,
        );
    }

    let payload = match req.json::<RegisterRequest>().await {
        Ok(v) => v,
        Err(_) => {
            return json_with_status(
                400,
                &ApiResponse {
                    success: false,
                    message: "请求体必须是 JSON，包含 username/password/invite_code".to_string(),
                },
                cors_origin,
            )
        }
    };

    let username = payload.username.trim().to_lowercase();
    let password = payload.password.trim().to_string();
    let invite_code = payload.invite_code.trim().to_string();
    let support_email = env
        .var("SUPPORT_EMAIL")
        .map(|v| v.to_string())
        .unwrap_or_else(|_| DEFAULT_SUPPORT_EMAIL.to_string());

    if username.len() < 3 || username.len() > 32 {
        return json_with_status(
            400,
            &ApiResponse {
                success: false,
                message: "用户名长度需在 3 到 32 之间".to_string(),
            },
            cors_origin,
        );
    }

    if password.len() < 8 {
        return json_with_status(
            400,
            &ApiResponse {
                success: false,
                message: "密码长度至少 8 位".to_string(),
            },
            cors_origin,
        );
    }

    if invite_code.is_empty() {
        return json_with_status(
            400,
            &ApiResponse {
                success: false,
                message: format!("注册需要邀请码，请发送邮件至 {} 获取邀请码", support_email),
            },
            cors_origin,
        );
    }

    let configured_invite_code = env
        .secret("INVITE_CODE")
        .map(|s| s.to_string())
        .or_else(|_| env.var("INVITE_CODE").map(|v| v.to_string()))
        .map_err(|_| Error::RustError("未配置 INVITE_CODE（Secret 或 Var）".to_string()))?;

    if invite_code != configured_invite_code {
        return json_with_status(
            403,
            &ApiResponse {
                success: false,
                message: format!("邀请码错误，请发送邮件至 {} 获取邀请码", support_email),
            },
            cors_origin,
        );
    }

    let db = env.d1(D1_BINDING)?;
    let exists_stmt = db
        .prepare("SELECT id FROM users WHERE username = ?1 LIMIT 1")
        .bind(&[JsValue::from_str(&username)])?;

    let existing: Option<ExistingUserRow> = exists_stmt.first(None).await?;
    if let Some(row) = existing {
        let _ = row.id;
        return json_with_status(
            409,
            &ApiResponse {
                success: false,
                message: "用户名已存在".to_string(),
            },
            cors_origin,
        );
    }

    let pepper = env
        .secret("PASSWORD_PEPPER")
        .map(|s| s.to_string())
        .or_else(|_| env.var("PASSWORD_PEPPER").map(|v| v.to_string()))
        .map_err(|_| Error::RustError("未配置 PASSWORD_PEPPER（Secret 或 Var）".to_string()))?;

    let password_hash = hash_password(&username, &password, &pepper);
    let created_at = now_iso();

    db.prepare("INSERT INTO users (username, password_hash, created_at) VALUES (?1, ?2, ?3)")
        .bind(&[
            JsValue::from_str(&username),
            JsValue::from_str(&password_hash),
            JsValue::from_str(&created_at),
        ])?
        .run()
        .await?;

    json_with_status(
        201,
        &ApiResponse {
            success: true,
            message: "注册成功".to_string(),
        },
        cors_origin,
    )
}

pub async fn login(mut req: Request, env: &Env, cors_origin: &str) -> Result<HttpResponse> {
    if let Err(msg) = apply_rate_limit(&req, env, "login").await {
        return json_with_status(
            429,
            &LoginResponse {
                success: false,
                message: msg,
                token: None,
                username: None,
            },
            cors_origin,
        );
    }

    let payload = match req.json::<LoginRequest>().await {
        Ok(v) => v,
        Err(_) => {
            return json_with_status(
                400,
                &LoginResponse {
                    success: false,
                    message: "请求体必须是 JSON，包含 username/password".to_string(),
                    token: None,
                    username: None,
                },
                cors_origin,
            )
        }
    };

    let username = payload.username.trim().to_lowercase();
    let password = payload.password.trim().to_string();

    let pepper = env
        .secret("PASSWORD_PEPPER")
        .map(|s| s.to_string())
        .or_else(|_| env.var("PASSWORD_PEPPER").map(|v| v.to_string()))
        .map_err(|_| Error::RustError("未配置 PASSWORD_PEPPER（Secret 或 Var）".to_string()))?;

    let db = env.d1(D1_BINDING)?;
    let stmt = db
        .prepare("SELECT id, username, password_hash FROM users WHERE username = ?1 LIMIT 1")
        .bind(&[JsValue::from_str(&username)])?;

    let user: Option<UserAuthRow> = stmt.first(None).await?;
    let Some(user) = user else {
        return json_with_status(
            401,
            &LoginResponse {
                success: false,
                message: "用户名或密码错误".to_string(),
                token: None,
                username: None,
            },
            cors_origin,
        );
    };

    let expected_hash = hash_password(&username, &password, &pepper);
    if expected_hash != user.password_hash {
        return json_with_status(
            401,
            &LoginResponse {
                success: false,
                message: "用户名或密码错误".to_string(),
                token: None,
                username: None,
            },
            cors_origin,
        );
    }

    let token = generate_session_token(&username, &pepper);
    let session_ttl: u64 = env
        .var("SESSION_TTL_SECS")
        .ok()
        .and_then(|v| v.to_string().parse().ok())
        .unwrap_or(86400);

    let kv = env
        .kv(KV_BINDING)
        .map_err(|_| Error::RustError("未配置 KV_LIMITER 绑定".to_string()))?;

    let session = SessionData {
        user_id: user.id,
        username: user.username.clone(),
        issued_at: now_iso(),
    };

    kv.put(&format!("session:{}", token), &session)
        .map_err(|e| Error::RustError(format!("KV 写入失败: {e}")))?
        .expiration_ttl(session_ttl)
        .execute()
        .await
        .map_err(|e| Error::RustError(format!("KV 写入失败: {e}")))?;

    json_with_status(
        200,
        &LoginResponse {
            success: true,
            message: "登录成功".to_string(),
            token: Some(token),
            username: Some(user.username),
        },
        cors_origin,
    )
}

pub async fn me(req: Request, env: &Env, cors_origin: &str) -> Result<HttpResponse> {
    let session = match require_auth(&req, env).await {
        Ok(s) => s,
        Err(resp) => return Ok(resp),
    };

    json_with_status(
        200,
        &serde_json::json!({
            "success": true,
            "user": {
                "id": session.user_id,
                "username": session.username,
                "issued_at": session.issued_at
            }
        }),
        cors_origin,
    )
}

pub async fn save_workspace(
    mut req: Request,
    env: &Env,
    cors_origin: &str,
) -> Result<HttpResponse> {
    let session = match require_auth(&req, env).await {
        Ok(s) => s,
        Err(resp) => return Ok(resp),
    };

    let payload = match req.json::<SaveWorkspaceRequest>().await {
        Ok(v) => v,
        Err(_) => {
            return json_with_status(
                400,
                &WorkspaceResponse {
                    success: false,
                    title: None,
                    payload: None,
                    updated_at: None,
                    message: Some("请求体必须是 JSON，包含 payload".to_string()),
                },
                cors_origin,
            )
        }
    };

    let title = payload
        .title
        .unwrap_or_else(|| "未命名文档".to_string())
        .trim()
        .to_string();
    let payload_str = serde_json::to_string(&payload.payload).unwrap_or_else(|_| "{}".to_string());
    let updated_at = now_iso();

    let db = env.d1(D1_BINDING)?;
    db.prepare(
        "INSERT INTO documents (user_id, title, payload, updated_at) VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(&[
        JsValue::from_f64(session.user_id as f64),
        JsValue::from_str(&title),
        JsValue::from_str(&payload_str),
        JsValue::from_str(&updated_at),
    ])?
    .run()
    .await?;

    json_with_status(
        200,
        &WorkspaceResponse {
            success: true,
            title: Some(title),
            payload: Some(payload.payload),
            updated_at: Some(updated_at),
            message: Some("保存成功".to_string()),
        },
        cors_origin,
    )
}

pub async fn latest_workspace(req: Request, env: &Env, cors_origin: &str) -> Result<HttpResponse> {
    let session = match require_auth(&req, env).await {
        Ok(s) => s,
        Err(resp) => return Ok(resp),
    };

    let db = env.d1(D1_BINDING)?;
    let stmt = db
        .prepare(
            "SELECT title, payload, updated_at FROM documents WHERE user_id = ?1 ORDER BY id DESC LIMIT 1",
        )
        .bind(&[JsValue::from_f64(session.user_id as f64)])?;

    let latest: Option<LatestDocRow> = stmt.first(None).await?;
    let Some(latest) = latest else {
        return json_with_status(
            404,
            &WorkspaceResponse {
                success: false,
                title: None,
                payload: None,
                updated_at: None,
                message: Some("暂无已保存文档".to_string()),
            },
            cors_origin,
        );
    };

    let payload = serde_json::from_str::<serde_json::Value>(&latest.payload)
        .unwrap_or_else(|_| serde_json::json!({}));

    json_with_status(
        200,
        &WorkspaceResponse {
            success: true,
            title: Some(latest.title),
            payload: Some(payload),
            updated_at: Some(latest.updated_at),
            message: None,
        },
        cors_origin,
    )
}

pub async fn generate_pdf(mut req: Request, env: &Env, cors_origin: &str) -> Result<HttpResponse> {
    let _session = match require_auth(&req, env).await {
        Ok(s) => s,
        Err(resp) => return Ok(resp),
    };

    let payload = match req.json::<PdfGenerateRequest>().await {
        Ok(v) => v,
        Err(_) => {
            return json_with_status(
                400,
                &ApiResponse {
                    success: false,
                    message: "请求体必须是 JSON，包含 diagnosis/prescription".to_string(),
                },
                cors_origin,
            )
        }
    };

    let title = payload
        .title
        .unwrap_or_else(|| "处方导出".to_string())
        .trim()
        .to_string();

    if let Some(image_data_url) = payload.image_data_url {
        let jpeg_bytes = decode_data_url_jpeg(&image_data_url)
            .ok_or_else(|| Error::RustError("图片数据无效，仅支持 JPEG data URL".to_string()))?;
        let (w, h) = jpeg_dimensions(&jpeg_bytes)
            .ok_or_else(|| Error::RustError("无法识别 JPEG 图片尺寸".to_string()))?;

        let (page_w, page_h) = match payload.page_mode.as_deref() {
            Some("diagnosis") => (595.0_f64, 842.0_f64),
            _ => (595.0_f64, 420.0_f64),
        };

        let pdf_bytes = build_image_pdf(&jpeg_bytes, w, h, page_w, page_h);

        let mut response = Response::from_bytes(pdf_bytes)?.with_status(200);
        let headers = response.headers_mut();
        headers.set("Content-Type", "application/pdf")?;
        headers.set(
            "Content-Disposition",
            "attachment; filename=prescription-export.pdf",
        )?;
        with_cors_headers(&mut response, cors_origin)?;

        return HttpResponse::try_from(response);
    }

    let diagnosis = payload.diagnosis.unwrap_or_default();
    let prescription = payload.prescription.unwrap_or_default();
    if diagnosis.trim().is_empty() && prescription.trim().is_empty() {
        return json_with_status(
            400,
            &ApiResponse {
                success: false,
                message: "缺少导出内容，请提供图片或处方文本".to_string(),
            },
            cors_origin,
        );
    }

    let mut lines = vec![
        format!("标题: {}", title),
        format!("导出时间: {}", now_iso()),
        "".to_string(),
        "诊断: ".to_string(),
    ];
    lines.extend(split_multiline(&diagnosis));
    lines.push("".to_string());
    lines.push("处方: ".to_string());
    lines.extend(split_multiline(&prescription));
    if let Some(note) = payload.note {
        lines.push("".to_string());
        lines.push("备注: ".to_string());
        lines.extend(split_multiline(&note));
    }

    let pdf_bytes = build_simple_pdf(&title, &lines);

    let mut response = Response::from_bytes(pdf_bytes)?.with_status(200);
    let headers = response.headers_mut();
    headers.set("Content-Type", "application/pdf")?;
    headers.set(
        "Content-Disposition",
        "attachment; filename=prescription-export.pdf",
    )?;
    with_cors_headers(&mut response, cors_origin)?;

    HttpResponse::try_from(response)
}

pub async fn generate_prescription(
    mut req: Request,
    env: &Env,
    cors_origin: &str,
) -> Result<HttpResponse> {
    let payload = match req.json::<GeneratePrescriptionRequest>().await {
        Ok(v) => v,
        Err(_) => {
            return json_with_status(
                400,
                &GeneratePrescriptionResponse {
                    success: false,
                    result: None,
                    message: Some("请求体必须是 JSON，包含 diag_text".to_string()),
                },
                cors_origin,
            )
        }
    };

    let diag_text = payload.diag_text.trim();
    if diag_text.is_empty() {
        return json_with_status(
            400,
            &GeneratePrescriptionResponse {
                success: false,
                result: None,
                message: Some("请先输入临床诊断".to_string()),
            },
            cors_origin,
        );
    }

    let deepseek_api_key = env
        .secret("DEEPSEEK_API_KEY")
        .map(|s| s.to_string())
        .or_else(|_| env.var("DEEPSEEK_API_KEY").map(|v| v.to_string()));
    let deepseek_api_key = match deepseek_api_key {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            return json_with_status(
                500,
                &GeneratePrescriptionResponse {
                    success: false,
                    result: None,
                    message: Some("服务端未配置 DEEPSEEK_API_KEY".to_string()),
                },
                cors_origin,
            )
        }
    };

    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Authorization", &format!("Bearer {}", deepseek_api_key))?;

    let payload = serde_json::json!({
        "model": "deepseek-chat",
        "temperature": 0.7,
        "messages": [
            { "role": "system", "content": "你是一个专业的医疗AI助手，输出格式严谨专业。" },
            { "role": "user", "content": build_prompt(diag_text) }
        ]
    });

    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(JsValue::from_str(&payload.to_string())));

    let outbound = Request::new_with_init("https://api.deepseek.com/chat/completions", &init)
        .map_err(|e| Error::RustError(format!("创建 DeepSeek 请求失败: {e}")))?;
    let mut upstream = Fetch::Request(outbound).send().await?;

    let upstream_status = upstream.status_code();
    let upstream_text = upstream.text().await.unwrap_or_default();
    let parsed = serde_json::from_str::<serde_json::Value>(&upstream_text)
        .unwrap_or_else(|_| serde_json::json!({}));

    if !(200..=299).contains(&upstream_status) {
        let msg = parsed
            .get("error")
            .and_then(|v| v.get("message"))
            .and_then(|v| v.as_str())
            .or_else(|| {
                if upstream_text.is_empty() {
                    None
                } else {
                    Some(upstream_text.as_str())
                }
            })
            .unwrap_or("DeepSeek 请求失败")
            .to_string();
        return json_with_status(
            upstream_status,
            &GeneratePrescriptionResponse {
                success: false,
                result: None,
                message: Some(format!("DeepSeek request failed: {}", msg)),
            },
            cors_origin,
        );
    }

    let result = parsed
        .get("choices")
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("message"))
        .and_then(|v| v.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    if result.is_empty() {
        return json_with_status(
            502,
            &GeneratePrescriptionResponse {
                success: false,
                result: None,
                message: Some("DeepSeek returned empty content".to_string()),
            },
            cors_origin,
        );
    }

    json_with_status(
        200,
        &GeneratePrescriptionResponse {
            success: true,
            result: Some(result),
            message: None,
        },
        cors_origin,
    )
}

fn build_prompt(diag_text: &str) -> String {
    format!(
        "你是一个非常专业且经验丰富的临床医生。请根据以下患者的【临床诊断】，为其开具一份合理的【处方及处理意见】。\n\n要求：\n1. 直接输出处方药品，不需要任何多余的开头和结尾问候语。\n2. 格式参照如下：\n1. 药品A (规格) 用法用量\n2. 药品B (规格) 用法用量\n你具体格式参考：\n1. 草酸艾司西酞普兰片(10mg/片) 50片/20mg Qd 口服。\n\n临床诊断：{}",
        diag_text
    )
}

pub fn prescription_random(cors_origin: &str) -> Result<HttpResponse> {
    let mut med_code = "H".to_string();
    for _ in 0..11 {
        med_code.push(char::from(b'0' + rand_digit() as u8));
    }

    let random_date = random_date_from_now();

    let year = random_date.get_full_year() as i32;
    let month = (random_date.get_month() + 1) as i32;
    let day = random_date.get_date() as i32;

    let mut rp = String::new();
    for _ in 0..5 {
        rp.push(char::from(b'0' + rand_digit() as u8));
    }

    let prescription_no = format!("{}{:02}{:02}{}", year, month, day, rp);
    let date_input = format!("{}-{:02}-{:02}", year, month, day);
    let outpatient_no = format!("MZ{}{}73", year % 100, rp);

    let hs = rand_range_int(8, 15);
    let ms = rand_range_int(0, 59);
    let ss = rand_range_int(0, 59);
    let ms2 = (ms + 15) % 60;

    let diag_date1 = format!("{}-{}-{} {:02}:{:02}:{:02}", year, month, day, hs, ms, ss);
    let diag_date2 = format!("{}-{}-{} {:02}:{:02}:{:02}", year, month, day, hs, ms2, ss);

    let a5_count = 35;
    let diag_count = 24;
    let mut a5_bar_widths = Vec::with_capacity(a5_count);
    let mut a5_bar_margins = Vec::with_capacity(a5_count);
    let mut diag_bar_widths = Vec::with_capacity(diag_count);
    let mut diag_bar_margins = Vec::with_capacity(diag_count);

    for _ in 0..a5_count {
        a5_bar_widths.push(rand_range_int(1, 3) as u8);
        a5_bar_margins.push(rand_range_int(1, 3) as u8);
    }
    for _ in 0..diag_count {
        diag_bar_widths.push(rand_range_int(1, 3) as u8);
        diag_bar_margins.push(rand_range_int(1, 2) as u8);
    }

    json_with_status(
        200,
        &RandomPayloadResponse {
            success: true,
            med_code,
            prescription_no,
            date_input,
            outpatient_no,
            diag_date1,
            diag_date2,
            a5_bar_widths,
            a5_bar_margins,
            diag_bar_widths,
            diag_bar_margins,
        },
        cors_origin,
    )
}
