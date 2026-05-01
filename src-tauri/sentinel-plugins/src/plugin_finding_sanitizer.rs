pub(crate) fn sanitize_response_body_for_evidence(
    response_headers: Option<&str>,
    response_body: String,
) -> Option<String> {
    if response_headers_indicate_non_text(response_headers)
        || contains_binary_text_markers(&response_body)
    {
        None
    } else {
        Some(response_body)
    }
}

fn response_headers_indicate_non_text(headers: Option<&str>) -> bool {
    let Some(headers) = headers else {
        return false;
    };

    if header_value(headers, "content-disposition")
        .map(|value| value.to_lowercase().contains("attachment"))
        .unwrap_or(false)
    {
        return true;
    }

    let Some(content_type) = header_value(headers, "content-type") else {
        return false;
    };
    let normalized = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    if normalized.is_empty() {
        return false;
    }

    !is_textual_content_type(&normalized)
}

fn is_textual_content_type(content_type: &str) -> bool {
    content_type.starts_with("text/")
        || content_type == "application/json"
        || content_type.ends_with("+json")
        || content_type == "application/xml"
        || content_type.ends_with("+xml")
        || content_type == "application/javascript"
        || content_type == "application/x-javascript"
        || content_type == "application/graphql"
}

fn header_value(headers: &str, name: &str) -> Option<String> {
    let normalized_name = name.to_lowercase();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(headers) {
        if let Some(items) = value.as_array() {
            for item in items {
                let header_name = item.get("name").and_then(|value| value.as_str());
                let header_value = item.get("value").and_then(|value| value.as_str());
                if header_name
                    .map(|header_name| header_name.eq_ignore_ascii_case(&normalized_name))
                    .unwrap_or(false)
                {
                    return header_value.map(ToString::to_string);
                }
            }
        }

        if let Some(object) = value.as_object() {
            for (header_name, header_value) in object {
                if header_name.eq_ignore_ascii_case(&normalized_name) {
                    return header_value.as_str().map(ToString::to_string);
                }
            }
        }
    }

    for line in headers.lines() {
        let Some((header_name, header_value)) = line.split_once(':') else {
            continue;
        };
        if header_name.trim().eq_ignore_ascii_case(&normalized_name) {
            return Some(header_value.trim().to_string());
        }
    }

    None
}

fn contains_binary_text_markers(value: &str) -> bool {
    let sample: String = value.chars().take(512).collect();
    if sample.is_empty() {
        return false;
    }

    let binary_markers = sample
        .chars()
        .filter(|ch| {
            matches!(
                *ch as u32,
                0x00..=0x08 | 0x0b | 0x0c | 0x0e..=0x1f | 0xfffd
            )
        })
        .count();

    binary_markers * 100 > sample.chars().count() * 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_response_body_for_evidence_drops_attachment_body() {
        let headers = r#"[{"name":"content-type","value":"application/octet-stream"},{"name":"content-disposition","value":"attachment; filename=\"video.mp4\""}]"#;

        assert_eq!(
            sanitize_response_body_for_evidence(Some(headers), "binary payload".to_string()),
            None
        );
    }

    #[test]
    fn test_sanitize_response_body_for_evidence_keeps_json_body() {
        let headers = r#"[{"name":"content-type","value":"application/json; charset=utf-8"}]"#;

        assert_eq!(
            sanitize_response_body_for_evidence(Some(headers), r#"{"ok":true}"#.to_string()),
            Some(r#"{"ok":true}"#.to_string())
        );
    }
}
