const RAG_KNOWLEDGE_POLICY: &str = "you must strictly answer the question based on the evidence. When citing evidence in your response, use the [SOURCE n] format. If the evidence is insufficient, please answer directly and avoid fabricating. ";

pub(crate) fn append_rag_knowledge_rule(base_system_prompt: Option<String>) -> String {
    let rule_block = format!("[rule of knowledge]\n{}", RAG_KNOWLEDGE_POLICY);

    match base_system_prompt {
        Some(base) if !base.trim().is_empty() => format!("{}\n\n{}", base.trim_end(), rule_block),
        _ => rule_block,
    }
}

pub(crate) fn prepend_rag_source_evidence(task: &str, evidence: &str) -> String {
    format!(
        "[Source Evidence Block]\n{}\n\n{}",
        evidence.trim(),
        task.trim_start()
    )
}

#[cfg(test)]
mod tests {
    use super::{append_rag_knowledge_rule, prepend_rag_source_evidence};

    #[test]
    fn rag_policy_is_system_rule_without_source_evidence() {
        let system_prompt = append_rag_knowledge_rule(Some("base rules".to_string()));

        assert!(system_prompt.contains("base rules"));
        assert!(system_prompt.contains("[rule of knowledge]"));
        assert!(!system_prompt.contains("[Source Evidence Block]"));
    }

    #[test]
    fn rag_source_evidence_is_runtime_task_context() {
        let task = prepend_rag_source_evidence("answer the question", "[SOURCE 1] fact");

        assert!(task.starts_with("[Source Evidence Block]\n[SOURCE 1] fact"));
        assert!(task.ends_with("answer the question"));
    }
}
