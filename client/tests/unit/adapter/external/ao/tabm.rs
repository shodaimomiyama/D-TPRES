use formix::adapter::external::ao::tabm;

#[test]
fn test_encode_multipart_simple_parts() {
    let result = tabm::encode_hb_multipart(&[("action", "Ping"), ("type", "Message")]);

    let body_str = String::from_utf8(result.body.clone()).unwrap();
    assert!(body_str.contains("action"));
    assert!(body_str.contains("Ping"));
    assert!(body_str.contains("type"));
    assert!(body_str.contains("Message"));
    assert!(result.content_type.starts_with("multipart/form-data; boundary=\""));
}

#[test]
fn test_encode_multipart_parts_sorted_lexicographically() {
    let result = tabm::encode_hb_multipart(&[("zebra", "z"), ("alpha", "a")]);
    let body_str = String::from_utf8(result.body.clone()).unwrap();

    let alpha_pos = body_str.find("alpha").unwrap();
    let zebra_pos = body_str.find("zebra").unwrap();
    assert!(alpha_pos < zebra_pos, "parts should be sorted lexicographically");
}

#[test]
fn test_encode_multipart_deterministic_boundary() {
    let parts = &[("key", "value")];
    let r1 = tabm::encode_hb_multipart(parts);
    let r2 = tabm::encode_hb_multipart(parts);
    assert_eq!(r1.boundary, r2.boundary, "boundary should be deterministic");
    assert_eq!(r1.body, r2.body, "body should be deterministic");
}

#[test]
fn test_encode_nested_body() {
    let result = tabm::encode_hb_nested_body(
        "device-stack",
        &[("1", "WASI@1.0"), ("2", "JSON-Iface@1.0")],
    );
    let body_str = String::from_utf8(result.body.clone()).unwrap();
    assert!(body_str.contains("device-stack"));
    assert!(body_str.contains("WASI@1.0"));
}

#[test]
fn test_encode_mixed_body() {
    let result = tabm::encode_hb_mixed_body(&[
        tabm::BodyPart::Nested {
            name: "device-stack",
            items: &[("1", "WASI@1.0")],
        },
        tabm::BodyPart::Binary {
            name: "body",
            content: b"hello",
        },
    ]);
    let body_str = String::from_utf8(result.body.clone()).unwrap();
    assert!(body_str.contains("body"));
    assert!(body_str.contains("device-stack"));
}

#[test]
fn test_multipart_boundary_format() {
    let result = tabm::encode_hb_multipart(&[("key", "value")]);
    let body_str = String::from_utf8(result.body.clone()).unwrap();
    assert!(body_str.starts_with(&format!("--{}", result.boundary)));
    assert!(body_str.ends_with(&format!("--{}--", result.boundary)));
}
