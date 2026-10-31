use constlit::{MediaType, mime};

#[test]
fn mime_in_const() {
    const JSON: MediaType = mime!("application/json");
    assert_eq!(JSON.essence(), "application/json");
    assert_eq!(JSON.subtype(), "json");
    assert_eq!(
        mime!("Text/HTML; Charset=UTF-8").param("charset"),
        Some("utf-8")
    );
}

#[cfg(feature = "mime")]
#[test]
fn to_mime() {
    assert_eq!(
        mime!("text/html; charset=utf-8").to_mime().unwrap(),
        mime::TEXT_HTML_UTF_8
    );
    assert_eq!(
        mime!(r#"multipart/form-data; boundary="a b""#)
            .to_mime()
            .unwrap()
            .get_param("boundary")
            .unwrap(),
        "a b"
    );
}

#[cfg(feature = "mime")]
#[test]
fn to_mime_unrepresentable() {
    assert!(mime!(r#"text/plain; a="""#).to_mime().is_err());
    assert!(mime!(r#"text/plain; a="x\"y""#).to_mime().is_err());
}
