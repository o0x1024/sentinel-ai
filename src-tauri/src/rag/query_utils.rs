use regex::Regex;

/// Build primary and fallback short queries for RAG based on a long, noisy input.
/// - primary: entity + security terms prioritized, length-limited
/// - fallback: ultra-short combo when primary yields no results (to be used by caller)
pub fn build_rag_query_pair(input: &str) -> (String, String) {
    let cleaned = normalize_input(input);
    let tokens = extract_signal_tokens(&cleaned);
    let primary = assemble_primary_query(&tokens, 10, 160);
    let fallback = assemble_fallback_query(&tokens, 6, 100);
    (primary, fallback)
}

fn normalize_input(s: &str) -> String {
    // Remove markdown fences, collapse whitespace
    let mut out = s
        .replace("```", " ")
        .replace("\n", " ")
        .replace("\r", " ")
        .replace("\t", " ");
    // Collapse multiple spaces
    out = Regex::new(r"\s+").unwrap().replace_all(&out, " ").to_string();
    out.trim().to_string()
}

fn extract_signal_tokens(s: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();

    // Regex patterns
    let cve_re = Regex::new(r"CVE-\d{4}-\d{4,7}").unwrap();
    let ip_re = Regex::new(r"\b(\d{1,3}\.){3}\d{1,3}\b").unwrap();
    let port_re = Regex::new(r"\b(\d{2,5})/(tcp|udp)\b").unwrap();
    let url_like_re = Regex::new(r"https?://[\w\.-/:]+").unwrap();

    // Dictionaries (small, extendable)
    let products = [
        "Shiro","Spring","Fastjson","Log4j","Apache","Nginx","Tomcat","Redis","MySQL","PostgreSQL","Docker","Kubernetes"
    ];
    let vul_terms = [
        "反序列化","RCE","SSRF","SQL注入","SQLi","XSS","CSRF","命令注入","漏洞","指纹","验证","POC","Exploit","Payload","修复","补丁"
    ];
    let en_vul_terms = [
        "deserialization","rce","ssrf","sqli","xss","csrf","injection","exploit","payload","patch","mitigation","advisory","fingerprint","verify"
    ];
    let stop_words = [
        // Chinese common stop words (subset)
        "请","根据","分析","生成","提供","任务","步骤","上下文","总结","如何","应该","是否","并","以及","进行","判断","目标","结果","返回","差异",
        // English stop words (subset)
        "the","and","or","for","to","of","with","on","in","by","an","a","is","are","be","should","could","would","please","based"
    ];

    // 1) Regex entities
    for m in cve_re.find_iter(s) { tokens.push(m.as_str().to_string()); }
    for m in ip_re.find_iter(s) { tokens.push(m.as_str().to_string()); }
    for m in port_re.find_iter(s) { tokens.push(m.as_str().to_string()); }
    for m in url_like_re.find_iter(s) { tokens.push(m.as_str().to_string()); }

    // 2) Dictionary-based terms (preserve order of appearance)
    let lower = s.to_lowercase();
    for p in &products {
        if s.contains(p) { tokens.push((*p).to_string()); }
    }
    for t in &vul_terms {
        if s.contains(t) { tokens.push((*t).to_string()); }
    }
    for t in &en_vul_terms {
        if lower.contains(t) { tokens.push((*t).to_string()); }
    }

    // 3) Split into words and keep non-stop alpha-numeric words (short Chinese keep as-is)
    for w in s.split(|c: char| c.is_whitespace() || ",.;:()[]{}<>\"'|".contains(c)) {
        if w.is_empty() { continue; }
        if stop_words.iter().any(|sw| sw.eq_ignore_ascii_case(w)) { continue; }
        if w.len() <= 1 { continue; }
        // prefer keeping ASCII words and meaningful Chinese chunks
        if w.chars().any(|c| c.is_alphanumeric()) || count_cjk(w) > 0 {
            tokens.push(w.to_string());
        }
    }

    // Deduplicate preserving order
    dedup_preserve_order(tokens)
}

fn assemble_primary_query(tokens: &[String], max_terms: usize, max_len: usize) -> String {
    assemble_query(tokens, max_terms, max_len)
}

fn assemble_fallback_query(tokens: &[String], max_terms: usize, max_len: usize) -> String {
    // Heuristic: prefer CVE/products + vul terms
    let mut prioritized: Vec<String> = Vec::new();
    let is_cve = |t: &str| t.starts_with("CVE-");
    let is_product = |t: &str| matches!(t, "Shiro"|"Spring"|"Fastjson"|"Log4j"|"Apache"|"Nginx"|"Tomcat"|"Redis"|"MySQL"|"PostgreSQL"|"Docker"|"Kubernetes");
    let is_vul = |t: &str| {
        [
            "反序列化","RCE","SSRF","SQL注入","SQLi","XSS","CSRF","命令注入","漏洞","指纹","验证",
            "deserialization","rce","ssrf","sqli","xss","csrf","injection","exploit","payload","patch","mitigation","advisory","fingerprint","verify"
        ].iter().any(|v| v.eq_ignore_ascii_case(t))
    };

    for t in tokens {
        if is_cve(t) || is_product(t) || is_vul(t) { prioritized.push(t.clone()); }
    }
    if prioritized.is_empty() { prioritized = tokens.to_vec(); }
    assemble_query(&prioritized, max_terms, max_len)
}

fn assemble_query(tokens: &[String], max_terms: usize, max_len: usize) -> String {
    let mut out: Vec<String> = Vec::new();
    for t in tokens {
        if out.len() >= max_terms { break; }
        out.push(t.clone());
        let s = out.join(" ");
        if s.len() > max_len {
            out.pop();
            break;
        }
    }
    out.join(" ")
}

fn count_cjk(s: &str) -> usize {
    s.chars().filter(|c| matches!(c, '\u{4E00}'..='\u{9FA5}' | '\u{3400}'..='\u{4DBF}')).count()
}

fn dedup_preserve_order(list: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for x in list.into_iter() {
        if seen.insert(x.clone()) { out.push(x); }
    }
    out
}


