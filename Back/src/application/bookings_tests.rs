use super::*;

#[test]
fn documents_are_recognised_by_their_first_bytes() {
    assert_eq!(
        document_kind(b"%PDF-1.7 ..."),
        Some(("pdf", "application/pdf"))
    );
    assert_eq!(
        document_kind(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A]),
        Some(("png", "image/png"))
    );
    assert_eq!(
        document_kind(&[0xFF, 0xD8, 0xFF, 0xE0]),
        Some(("jpg", "image/jpeg"))
    );
    assert_eq!(
        document_kind(b"RIFF\x10\x00\x00\x00WEBPVP8 "),
        Some(("webp", "image/webp"))
    );
    assert_eq!(document_kind(b"MZ\x90\x00"), None);
}

#[test]
fn tokens_are_long_and_url_safe() {
    let token = new_token();
    assert_eq!(token.len(), 43);
    assert!(
        token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    );
    assert_ne!(token, new_token());
}
