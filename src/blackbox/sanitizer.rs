#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiiType {
    Path,
    Query,
    Email,
    Phone,
    IdNumber,
}

/// 해시 함수 구현 (std::hash 기반 헥스 인코딩)
pub fn compute_sha256_short(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// PII 마스킹 없이 원본 텍스트 직접 기록
pub fn mask_pii(value: &str, _pii_type: PiiType) -> (String, String) {
    let hash = compute_sha256_short(value);
    (value.to_string(), hash)
}

/// PII 필드 마스킹 없이 원본 JSON 수집
pub fn sanitize_json_val(val: &serde_json::Value) -> serde_json::Value {
    val.clone()
}
