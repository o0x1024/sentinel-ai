use std::collections::HashSet;

pub fn score_skill_match(task_lower: &str, skill_name: &str, description: &str) -> usize {
    if task_lower.trim().is_empty() {
        return 0;
    }

    let mut score = 0usize;
    let name_lower = skill_name.to_lowercase();
    let desc_lower = description.to_lowercase();

    if task_lower.contains(&name_lower) {
        score += 8;
    }

    let mut keywords = HashSet::new();
    for part in name_lower
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .chain(desc_lower.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-'))
    {
        let token = part.trim();
        if token.len() >= 3 && token.len() <= 32 {
            keywords.insert(token.to_string());
        }
        if keywords.len() >= 36 {
            break;
        }
    }

    for token in keywords {
        if task_lower.contains(&token) {
            score += 1;
        }
    }

    score
}

pub fn extract_workflow_tags(name: &str, description: Option<&str>) -> Vec<String> {
    let mut tags = Vec::new();

    let name_lower = name.to_lowercase();
    let name_words: Vec<&str> = name_lower.split(|c: char| !c.is_alphanumeric()).collect();
    for word in name_words {
        if word.len() > 2 {
            tags.push(word.to_string());
        }
    }

    if let Some(desc) = description {
        let desc_lower = desc.to_lowercase();
        let keywords = [
            "scan",
            "test",
            "analyze",
            "report",
            "monitor",
            "alert",
            "security",
            "vulnerability",
            "penetration",
            "reconnaissance",
            "扫描",
            "测试",
            "分析",
            "报告",
            "监控",
            "告警",
            "安全",
            "漏洞",
        ];

        for keyword in keywords {
            if desc_lower.contains(keyword) {
                tags.push(keyword.to_string());
            }
        }
    }

    tags.sort();
    tags.dedup();
    tags
}

pub fn extract_mcp_tool_tags(name: &str, description: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let name_lower = name.to_lowercase();
    let name_words: Vec<&str> = name_lower.split(|c: char| !c.is_alphanumeric()).collect();

    for word in name_words {
        if word.len() > 2 {
            tags.push(word.to_string());
        }
    }

    let desc_lower = description.to_lowercase();
    let common_keywords = [
        "search", "read", "write", "edit", "run", "query", "fetch", "create", "update", "delete",
        "browser", "file", "database", "network",
    ];

    for keyword in common_keywords {
        if desc_lower.contains(keyword) {
            tags.push(keyword.to_string());
        }
    }

    tags.sort();
    tags.dedup();
    tags
}
