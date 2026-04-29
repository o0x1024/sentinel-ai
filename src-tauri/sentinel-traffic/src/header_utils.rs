use http::{request, response};
use std::collections::HashMap;

const SET_COOKIE_SEPARATOR: char = '\n';

pub fn merge_header_value(headers: &mut HashMap<String, String>, name: &str, value: &str) {
    match headers.get_mut(name) {
        Some(existing) => {
            let delimiter = if name.eq_ignore_ascii_case("cookie") {
                "; "
            } else if name.eq_ignore_ascii_case("set-cookie") {
                "\n"
            } else {
                ", "
            };
            existing.push_str(delimiter);
            existing.push_str(value);
        }
        None => {
            headers.insert(name.to_string(), value.to_string());
        }
    }
}

pub fn append_request_headers(
    mut builder: request::Builder,
    headers: &HashMap<String, String>,
) -> request::Builder {
    for (name, value) in headers {
        for segment in header_value_segments(name, value) {
            builder = builder.header(name, segment);
        }
    }

    builder
}

pub fn append_response_headers(
    mut builder: response::Builder,
    headers: &HashMap<String, String>,
) -> response::Builder {
    for (name, value) in headers {
        for segment in header_value_segments(name, value) {
            builder = builder.header(name, segment);
        }
    }

    builder
}

fn header_value_segments<'a>(name: &str, value: &'a str) -> Vec<&'a str> {
    if !name.eq_ignore_ascii_case("set-cookie") {
        return vec![value];
    }

    value
        .split(SET_COOKIE_SEPARATOR)
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{append_response_headers, merge_header_value};
    use http::Response;
    use std::collections::HashMap;

    #[test]
    fn appends_set_cookie_headers_as_multiple_values() {
        let mut headers = HashMap::new();
        merge_header_value(&mut headers, "set-cookie", "session=abc; Path=/; HttpOnly");
        merge_header_value(&mut headers, "set-cookie", "theme=dark; Path=/");

        let response = append_response_headers(Response::builder().status(200), &headers)
            .body(())
            .expect("response should be built successfully");

        let values: Vec<_> = response
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|value| value.to_str().expect("header should be valid").to_string())
            .collect();

        assert_eq!(
            values,
            vec![
                "session=abc; Path=/; HttpOnly".to_string(),
                "theme=dark; Path=/".to_string(),
            ]
        );
    }
}
