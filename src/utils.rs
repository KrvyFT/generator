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

pub fn split_multiline(input: &str) -> Vec<String> {
    input
        .split('\n')
        .map(|line| line.trim_end().to_string())
        .collect()
}

fn utf16be_hex(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 4);
    for unit in s.encode_utf16() {
        out.push_str(&format!("{:04X}", unit));
    }
    out
}

pub fn build_simple_pdf(title: &str, lines: &[String]) -> Vec<u8> {
    let mut content = String::new();
    content.push_str("BT\n/F1 12 Tf\n50 800 Td\n");
    content.push_str(&format!("<{}> Tj\n", utf16be_hex(title)));
    content.push_str("0 -18 Td\n");

    for line in lines.iter().take(48) {
        content.push_str(&format!("<{}> Tj\nT*\n", utf16be_hex(line)));
    }
    content.push_str("ET\n");

    let obj1 = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_string();
    let obj2 = "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_string();
    let obj3 = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n".to_string();
    let obj4 = format!(
        "4 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
        content.len(),
        content
    );
    let obj5 = "5 0 obj\n<< /Type /Font /Subtype /Type0 /BaseFont /STSong-Light /Encoding /UniGB-UCS2-H /DescendantFonts [6 0 R] >>\nendobj\n".to_string();
    let obj6 = "6 0 obj\n<< /Type /Font /Subtype /CIDFontType0 /BaseFont /STSong-Light /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 4 >> /DW 1000 >>\nendobj\n".to_string();

    let mut pdf = Vec::<u8>::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");

    let offsets = {
        let mut offsets = Vec::new();
        offsets.push(pdf.len());
        pdf.extend_from_slice(obj1.as_bytes());
        offsets.push(pdf.len());
        pdf.extend_from_slice(obj2.as_bytes());
        offsets.push(pdf.len());
        pdf.extend_from_slice(obj3.as_bytes());
        offsets.push(pdf.len());
        pdf.extend_from_slice(obj4.as_bytes());
        offsets.push(pdf.len());
        pdf.extend_from_slice(obj5.as_bytes());
        offsets.push(pdf.len());
        pdf.extend_from_slice(obj6.as_bytes());
        offsets
    };

    let xref_pos = pdf.len();
    pdf.extend_from_slice(b"xref\n0 7\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }

    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            xref_pos
        )
        .as_bytes(),
    );

    pdf
}

pub fn decode_data_url_jpeg(input: &str) -> Option<Vec<u8>> {
    let prefix = "data:image/jpeg;base64,";
    let payload = input.strip_prefix(prefix)?;
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .ok()
}

pub fn jpeg_dimensions(bytes: &[u8]) -> Option<(u16, u16)> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return None;
    }

    let mut i = 2usize;
    while i + 9 < bytes.len() {
        if bytes[i] != 0xFF {
            i += 1;
            continue;
        }
        let marker = bytes[i + 1];
        i += 2;

        if marker == 0xD8 || marker == 0xD9 {
            continue;
        }
        if i + 1 >= bytes.len() {
            break;
        }

        let seg_len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
        if seg_len < 2 || i + seg_len > bytes.len() {
            break;
        }

        let is_sof = matches!(
            marker,
            0xC0 | 0xC1
                | 0xC2
                | 0xC3
                | 0xC5
                | 0xC6
                | 0xC7
                | 0xC9
                | 0xCA
                | 0xCB
                | 0xCD
                | 0xCE
                | 0xCF
        );

        if is_sof && seg_len >= 7 {
            let h = u16::from_be_bytes([bytes[i + 3], bytes[i + 4]]);
            let w = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]);
            return Some((w, h));
        }

        i += seg_len;
    }

    None
}

pub fn build_image_pdf(
    jpeg_bytes: &[u8],
    width: u16,
    height: u16,
    page_w: f64,
    page_h: f64,
) -> Vec<u8> {
    let img_w = width as f64;
    let img_h = height as f64;
    let scale = (page_w / img_w).min(page_h / img_h);
    let draw_w = (img_w * scale).max(1.0);
    let draw_h = (img_h * scale).max(1.0);
    let offset_x = (page_w - draw_w) / 2.0;
    let offset_y = (page_h - draw_h) / 2.0;

    let content = format!(
        "q\n{:.2} 0 0 {:.2} {:.2} {:.2} cm\n/Im1 Do\nQ\n",
        draw_w, draw_h, offset_x, offset_y
    );

    let obj1 = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_string();
    let obj2 = "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_string();
    let obj3 = format!(
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >>\nendobj\n",
        page_w, page_h
    );
    let obj4 = format!(
        "4 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
        content.len(),
        content
    );
    let obj5_header = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
        width,
        height,
        jpeg_bytes.len()
    );
    let obj5_footer = "\nendstream\nendobj\n";

    let mut pdf = Vec::<u8>::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");

    let mut offsets = Vec::new();
    offsets.push(pdf.len());
    pdf.extend_from_slice(obj1.as_bytes());
    offsets.push(pdf.len());
    pdf.extend_from_slice(obj2.as_bytes());
    offsets.push(pdf.len());
    pdf.extend_from_slice(obj3.as_bytes());
    offsets.push(pdf.len());
    pdf.extend_from_slice(obj4.as_bytes());
    offsets.push(pdf.len());
    pdf.extend_from_slice(obj5_header.as_bytes());
    pdf.extend_from_slice(jpeg_bytes);
    pdf.extend_from_slice(obj5_footer.as_bytes());

    let xref_pos = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }

    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            xref_pos
        )
        .as_bytes(),
    );

    pdf
}

pub fn random_date_from_now() -> Date {
    let now = Date::new_0();
    let offset_days = rand_range_int(0, 89) as f64;
    let random_ts = now.get_time() - (offset_days * 24.0 * 60.0 * 60.0 * 1000.0);
    Date::new(&JsValue::from_f64(random_ts))
}

#[cfg(test)]
mod tests {
    use super::build_simple_pdf;

    #[test]
    fn pdf_contains_cjk_font_and_utf16be_text() {
        let title = "处方导出";
        let lines = vec![
            "诊断: 重度抑郁".to_string(),
            "处方: 草酸艾司西酞普兰".to_string(),
        ];
        let bytes = build_simple_pdf(title, &lines);
        let text = String::from_utf8_lossy(&bytes);

        assert!(text.contains("/BaseFont /STSong-Light"));
        assert!(text.contains("/Encoding /UniGB-UCS2-H"));
        assert!(text.contains("<590465B95BFC51FA>"));
        assert!(text.contains("<8BCA65AD003A002091CD5EA6629190C1>"));
    }
}
