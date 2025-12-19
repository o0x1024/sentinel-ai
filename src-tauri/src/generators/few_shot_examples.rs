//! Few-shot learning examples for plugin generation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Few-shot example for plugin generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewShotExample {
    pub vuln_type: String,
    pub context: String,
    pub code: String,
    pub quality_score: f32,
}

/// Few-shot example repository
pub struct FewShotRepository {
    examples: HashMap<String, Vec<FewShotExample>>,
}

impl FewShotRepository {
    pub fn new() -> Self {
        let mut repo = Self {
            examples: HashMap::new(),
        };
        repo.load_builtin_examples();
        repo
    }

    /// Get examples for a specific vulnerability type
    pub fn get_examples(&self, vuln_type: &str) -> Vec<&FewShotExample> {
        self.examples
            .get(vuln_type)
            .map(|examples| examples.iter().collect())
            .unwrap_or_default()
    }

    /// Add a new example to the repository
    pub fn add_example(&mut self, example: FewShotExample) {
        self.examples
            .entry(example.vuln_type.clone())
            .or_insert_with(Vec::new)
            .push(example);
    }

    /// Load built-in high-quality examples
    fn load_builtin_examples(&mut self) {
        // SQL Injection Example
        self.add_example(FewShotExample {
            vuln_type: "sqli".to_string(),
            context: "MySQL database, REST API with numeric user_id parameter".to_string(),
            quality_score: 90.0,
            code: r#""#.to_string(),
        });

        // XSS Example
        self.add_example(FewShotExample {
            vuln_type: "xss".to_string(),
            context: "Express.js application with user comments feature".to_string(),
            quality_score: 88.0,
            code: r#""#.to_string(),
        });

        // IDOR Example
        self.add_example(FewShotExample {
            vuln_type: "idor".to_string(),
            context: "REST API with user profile access by ID".to_string(),
            quality_score: 85.0,
            code: r#"// High-quality IDOR detector"#.to_string(),
        });

        // Add more examples for other vulnerability types...
    }
}

impl Default for FewShotRepository {
    fn default() -> Self {
        Self::new()
    }
}
