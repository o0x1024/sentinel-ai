//! Input normalization helpers for interactive terminal commands.

/// Decode a small, explicit set of HTML entities that commonly appear when a
/// shell command is accidentally routed through an HTML-rendering layer.
///
/// We intentionally keep this narrow so interactive_shell only repairs common
/// transport corruption such as `&lt;`, `&gt;`, and `&amp;`.
pub fn decode_transport_html_entities(input: &str) -> String {
    let mut current = input.to_string();

    // Allow a couple of passes so `&amp;gt;` can recover to `>`, while still
    // keeping the transformation bounded and predictable.
    for _ in 0..3 {
        let decoded = current
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&amp;", "&");

        if decoded == current {
            break;
        }

        current = decoded;
    }

    current
}

#[cfg(test)]
mod tests {
    use super::decode_transport_html_entities;

    #[test]
    fn decodes_common_html_entities() {
        assert_eq!(
            decode_transport_html_entities("cat &gt; /tmp/x &lt;&lt; 'EOF'"),
            "cat > /tmp/x << 'EOF'"
        );
        assert_eq!(
            decode_transport_html_entities("echo &quot;ok&quot; &amp;&amp; echo &#39;done&#39;"),
            "echo \"ok\" && echo 'done'"
        );
    }

    #[test]
    fn decodes_double_escaped_entities_with_bounded_passes() {
        assert_eq!(
            decode_transport_html_entities("printf '&amp;lt;tag&amp;gt;'"),
            "printf '<tag>'"
        );
    }

    #[test]
    fn leaves_plain_shell_text_unchanged() {
        let command = "php -r 'echo $HOME;' > /tmp/out";
        assert_eq!(decode_transport_html_entities(command), command);
    }
}
