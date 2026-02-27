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
    Ok(())
}
