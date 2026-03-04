use worker::*;

pub fn empty_with_status(status: u16, cors_origin: &str) -> Result<HttpResponse> {
    let mut response = Response::empty()?.with_status(status);
    with_cors_headers(&mut response, cors_origin)?;
    HttpResponse::try_from(response)
}

pub fn html_with_status(status: u16, html: &str, cors_origin: &str) -> Result<HttpResponse> {
    let mut response = Response::from_html(html)?.with_status(status);
    with_cors_headers(&mut response, cors_origin)?;
    HttpResponse::try_from(response)
}

pub fn text_with_status(status: u16, text: &str, cors_origin: &str) -> Result<HttpResponse> {
    let mut response = Response::ok(text)?.with_status(status);
    with_cors_headers(&mut response, cors_origin)?;
    response
        .headers_mut()
        .set("Content-Type", "text/plain; charset=utf-8")?;
    response.headers_mut().set("Cache-Control", "no-store")?;
    HttpResponse::try_from(response)
}

pub fn json_with_status<T: serde::Serialize>(
    status: u16,
    body: &T,
    cors_origin: &str,
) -> Result<HttpResponse> {
    let mut response = Response::from_json(body)?.with_status(status);
    with_cors_headers(&mut response, cors_origin)?;
    HttpResponse::try_from(response)
}

pub fn with_cors_headers(response: &mut Response, cors_origin: &str) -> Result<()> {
    let headers = response.headers_mut();
    headers.set("Access-Control-Allow-Origin", cors_origin)?;
    headers.set("Access-Control-Allow-Methods", "POST, GET, OPTIONS")?;
    headers.set(
        "Access-Control-Allow-Headers",
        "Content-Type, Authorization",
    )?;
    headers.set("Vary", "Origin")?;
    headers.set("X-Content-Type-Options", "nosniff")?;
    headers.set("Referrer-Policy", "no-referrer")?;
    headers.set("X-Frame-Options", "SAMEORIGIN")?;
    headers.set(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains",
    )?;
    headers.set(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self' 'unsafe-inline' blob: https://cdn.jsdelivr.net; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; img-src 'self' data: blob:; font-src 'self' data: https://fonts.gstatic.com; connect-src 'self'; object-src 'none'; base-uri 'self'; frame-ancestors 'self'; upgrade-insecure-requests",
    )?;
    Ok(())
}
