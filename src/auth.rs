use axum::http::HeaderMap;

/// 从指定请求头中提取 Bearer token.
///
/// 请求头值必须形如 `Bearer <token>`.
pub fn extract_bearer_token<'a>(headers: &'a HeaderMap, header_name: &str) -> Option<&'a str> {
    let header = headers.get(header_name)?.to_str().ok()?;
    header.strip_prefix("Bearer ")
}
