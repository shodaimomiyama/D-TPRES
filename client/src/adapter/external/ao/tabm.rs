use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};

pub struct HbMultipart {
    pub body: Vec<u8>,
    pub boundary: String,
    pub content_type: String,
}

pub fn encode_hb_nested_body(part_name: &str, items: &[(&str, &str)]) -> HbMultipart {
    let disposition = format!("form-data;name=\"{part_name}\"");

    let mut kv: Vec<(&str, String)> = items.iter().map(|(k, v)| (*k, v.to_string())).collect();
    kv.push(("content-disposition", disposition));
    kv.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));

    let part_bytes: Vec<u8> = kv
        .iter()
        .map(|(k, v)| format!("{k}: {v}"))
        .collect::<Vec<_>>()
        .join("\r\n")
        .into_bytes();

    assemble_multipart(&[(part_name.to_string(), part_bytes)])
}

pub fn encode_hb_multipart(parts: &[(&str, &str)]) -> HbMultipart {
    let mut encoded: Vec<(String, Vec<u8>)> = parts
        .iter()
        .map(|(name, value)| {
            let key = name.to_lowercase();
            let part = format!("content-disposition: form-data;name=\"{key}\"\r\n\r\n{value}");
            (key, part.into_bytes())
        })
        .collect();

    encoded.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    assemble_multipart(&encoded)
}

pub enum BodyPart<'a> {
    Nested {
        name: &'a str,
        items: &'a [(&'a str, &'a str)],
    },
    Binary {
        name: &'a str,
        content: &'a [u8],
    },
}

pub fn encode_hb_mixed_body(parts: &[BodyPart]) -> HbMultipart {
    let mut encoded: Vec<(String, Vec<u8>)> = parts
        .iter()
        .map(|part| match part {
            BodyPart::Nested { name, items } => {
                let disposition = format!("form-data;name=\"{name}\"");
                let mut kv: Vec<(&str, String)> =
                    items.iter().map(|(k, v)| (*k, v.to_string())).collect();
                kv.push(("content-disposition", disposition));
                kv.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
                let bytes: Vec<u8> = kv
                    .iter()
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect::<Vec<_>>()
                    .join("\r\n")
                    .into_bytes();
                (name.to_string(), bytes)
            }
            BodyPart::Binary { name, content } => {
                let header = format!("content-disposition: form-data;name=\"{name}\"\r\n\r\n");
                let mut bytes = header.into_bytes();
                bytes.extend_from_slice(content);
                (name.to_string(), bytes)
            }
        })
        .collect();

    encoded.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    assemble_multipart(&encoded)
}

fn assemble_multipart(sorted_parts: &[(String, Vec<u8>)]) -> HbMultipart {
    let part_bytes: Vec<&[u8]> = sorted_parts.iter().map(|(_, p)| p.as_slice()).collect();

    let mut hasher = Sha256::new();
    for (i, part) in part_bytes.iter().enumerate() {
        if i > 0 {
            hasher.update(b"\r\n");
        }
        hasher.update(part);
    }
    let boundary = URL_SAFE_NO_PAD.encode(hasher.finalize());

    let mut body = Vec::new();
    for (i, part) in part_bytes.iter().enumerate() {
        if i == 0 {
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        } else {
            body.extend_from_slice(format!("\r\n--{boundary}\r\n").as_bytes());
        }
        body.extend_from_slice(part);
    }
    body.extend_from_slice(format!("\r\n--{boundary}--").as_bytes());

    let content_type = format!("multipart/form-data; boundary=\"{boundary}\"");

    HbMultipart {
        body,
        boundary,
        content_type,
    }
}
