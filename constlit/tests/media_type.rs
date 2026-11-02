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

#[test]
fn equality_follows_parameter_order() {
    assert_eq!(mime!("text/plain; a=1; b=2"), mime!("Text/Plain; A=1; B=2"));
    assert_ne!(mime!("text/plain; a=1; b=2"), mime!("text/plain; b=2; a=1"));
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
    assert!(mime!("text/plain; a=\"x\ty\"").to_mime().is_err());
}
