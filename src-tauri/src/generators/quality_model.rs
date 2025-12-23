//! Quality model for plugin code assessment

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Training sample for quality model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    /// Plugin code
    pub code: String,
    /// Actual quality score (from human review)
    pub actual_score: f32,
    /// Vulnerability type
    pub vuln_type: String,
    /// Code features
    pub features: CodeFeatures,
}

/// Code features for quality assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeFeatures {
    /// Lines of code
    pub loc: usize,
    /// Number of functions
    pub function_count: usize,
    /// Has comments
    pub has_comments: bool,
    /// Has type annotations
    pub has_types: bool,
    /// Has error handling
    pub has_error_handling: bool,
    /// Complexity score (0-100)
    pub complexity: f32,
    /// Number of payloads/test cases
    pub payload_count: usize,
    /// Uses regex
    pub uses_regex: bool,
}

/// Quality prediction model
pub struct QualityModel {
    /// Training samples
    samples: Vec<TrainingSample>,
    /// Feature weights (learned from training)
    weights: HashMap<String, f32>,
    /// Model version
    version: String,
}

impl QualityModel {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            weights: Self::default_weights(),
            version: "1.0.0".to_string(),
        }
    }
    
    /// Default feature weights (before training)
    fn default_weights() -> HashMap<String, f32> {
        let mut weights = HashMap::new();
        weights.insert("loc".to_string(), 0.1);
        weights.insert("function_count".to_string(), 0.15);
        weights.insert("has_comments".to_string(), 0.1);
        weights.insert("has_types".to_string(), 0.15);
        weights.insert("has_error_handling".to_string(), 0.2);
        weights.insert("complexity".to_string(), -0.1); // High complexity is bad
        weights.insert("payload_count".to_string(), 0.2);
        weights.insert("uses_regex".to_string(), 0.1);
        weights
    }
    
    /// Add training sample
    pub fn add_sample(&mut self, sample: TrainingSample) {
        self.samples.push(sample);
    }
    
    /// Train the model using collected samples
    pub fn train(&mut self) -> Result<TrainingReport> {
        if self.samples.is_empty() {
            return Err(anyhow::anyhow!("No training samples available"));
        }
        
        log::info!("Training quality model with {} samples", self.samples.len());
        
        // Simple linear regression approach
        // For each feature, calculate correlation with actual quality score
        let mut new_weights = HashMap::new();
        
        // Calculate mean quality score
        let mean_quality: f32 = self.samples.iter()
            .map(|s| s.actual_score)
            .sum::<f32>() / self.samples.len() as f32;
        
        // For each feature, calculate weight based on correlation
        let features = vec![
            "loc", "function_count", "has_comments", "has_types",
            "has_error_handling", "complexity", "payload_count", "uses_regex"
        ];
        
        for feature_name in features {
            let weight = self.calculate_feature_weight(feature_name, mean_quality);
            new_weights.insert(feature_name.to_string(), weight);
        }
        
        self.weights = new_weights;
        
        // Calculate training metrics
        let mut predictions = Vec::new();
        for sample in &self.samples {
            let predicted = self.predict(&sample.features)?;
            predictions.push((predicted, sample.actual_score));
        }
        
        let mse = Self::calculate_mse(&predictions);
        let mae = Self::calculate_mae(&predictions);
        let r2 = Self::calculate_r2(&predictions, mean_quality);
        
        log::info!(
            "Training complete. MSE: {:.2}, MAE: {:.2}, R²: {:.3}",
            mse, mae, r2
        );
        
        Ok(TrainingReport {
            samples_count: self.samples.len(),
            mse,
            mae,
            r2_score: r2,
            weights: self.weights.clone(),
            version: self.version.clone(),
        })
    }
    
    /// Calculate feature weight based on correlation with quality
    fn calculate_feature_weight(&self, feature_name: &str, _mean_quality: f32) -> f32 {
        let mut sum_xy = 0.0;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_x2 = 0.0;
        let mut sum_y2 = 0.0;
        let n = self.samples.len() as f32;
        
        for sample in &self.samples {
            let x = self.extract_feature_value(&sample.features, feature_name);
            let y = sample.actual_score;
            
            sum_xy += x * y;
            sum_x += x;
            sum_y += y;
            sum_x2 += x * x;
            sum_y2 += y * y;
        }
        
        // Pearson correlation coefficient
        let numerator = n * sum_xy - sum_x * sum_y;
        let denominator = ((n * sum_x2 - sum_x * sum_x) * (n * sum_y2 - sum_y * sum_y)).sqrt();
        
        if denominator == 0.0 {
            return 0.0;
        }
        
        let correlation = numerator / denominator;
        
        // Scale correlation to weight (0.0 to 0.3)
        correlation.abs() * 0.3
    }
    
    /// Extract numeric value from feature
    fn extract_feature_value(&self, features: &CodeFeatures, name: &str) -> f32 {
        match name {
            "loc" => (features.loc as f32).min(500.0) / 500.0 * 100.0,
            "function_count" => (features.function_count as f32).min(10.0) / 10.0 * 100.0,
            "has_comments" => if features.has_comments { 100.0 } else { 0.0 },
            "has_types" => if features.has_types { 100.0 } else { 0.0 },
            "has_error_handling" => if features.has_error_handling { 100.0 } else { 0.0 },
            "complexity" => features.complexity,
            "payload_count" => (features.payload_count as f32).min(20.0) / 20.0 * 100.0,
            "uses_regex" => if features.uses_regex { 100.0 } else { 0.0 },
            _ => 0.0,
        }
    }
    
    /// Predict quality score for given features
    pub fn predict(&self, features: &CodeFeatures) -> Result<f32> {
        let mut score = 0.0;
        
        for (feature_name, weight) in &self.weights {
            let feature_value = self.extract_feature_value(features, feature_name);
            score += feature_value * weight;
        }
        
        // Normalize to 0-100 range
        Ok(score.max(0.0).min(100.0))
    }
    
    /// Extract features from code
    pub fn extract_features(code: &str) -> CodeFeatures {
        let lines: Vec<&str> = code.lines().collect();
        let loc = lines.len();
        
        // Count functions
        let function_count = code.matches("function ").count() 
            + code.matches("async function ").count()
            + code.matches("=> {").count();
        
        // Check for comments
        let has_comments = code.contains("//") || code.contains("/*");
        
        // Check for type annotations
        let has_types = code.contains(": string") 
            || code.contains(": number")
            || code.contains(": boolean")
            || code.contains("interface ")
            || code.contains("type ");
        
        // Check for error handling
        let has_error_handling = code.contains("try {") 
            || code.contains("catch (")
            || code.contains(".catch(");
        
        // Estimate complexity (simple heuristic)
        let complexity = Self::calculate_complexity(code);
        
        // Count payloads/test cases
        let payload_count = code.matches("payload").count()
            + code.matches("test").count();
        
        // Check for regex usage
        let uses_regex = code.contains("new RegExp") || code.contains("/\\w+/");
        
        CodeFeatures {
            loc,
            function_count,
            has_comments,
            has_types,
            has_error_handling,
            complexity,
            payload_count,
            uses_regex,
        }
    }
    
    /// Calculate code complexity (cyclomatic complexity approximation)
    fn calculate_complexity(code: &str) -> f32 {
        let decision_points = code.matches("if (").count()
            + code.matches("for (").count()
            + code.matches("while (").count()
            + code.matches("case ").count()
            + code.matches("&&").count()
            + code.matches("||").count();
        
        // Normalize to 0-100 scale
        (decision_points as f32).min(50.0) / 50.0 * 100.0
    }
    
    /// Calculate Mean Squared Error
    fn calculate_mse(predictions: &[(f32, f32)]) -> f32 {
        let sum: f32 = predictions.iter()
            .map(|(pred, actual)| (pred - actual).powi(2))
            .sum();
        sum / predictions.len() as f32
    }
    
    /// Calculate Mean Absolute Error
    fn calculate_mae(predictions: &[(f32, f32)]) -> f32 {
        let sum: f32 = predictions.iter()
            .map(|(pred, actual)| (pred - actual).abs())
            .sum();
        sum / predictions.len() as f32
    }
    
    /// Calculate R² score
    fn calculate_r2(predictions: &[(f32, f32)], mean_actual: f32) -> f32 {
        let ss_res: f32 = predictions.iter()
            .map(|(pred, actual)| (actual - pred).powi(2))
            .sum();
        
        let ss_tot: f32 = predictions.iter()
            .map(|(_, actual)| (actual - mean_actual).powi(2))
            .sum();
        
        if ss_tot == 0.0 {
            return 0.0;
        }
        
        1.0 - (ss_res / ss_tot)
    }
    
    /// Save model to file
    pub fn save(&self, path: &str) -> Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }
    
    /// Load model from file
    pub fn load(path: &str) -> Result<Self> {
        let data = std::fs::read_to_string(path)?;
        let model = serde_json::from_str(&data)?;
        Ok(model)
    }
}

/// Training report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingReport {
    pub samples_count: usize,
    pub mse: f32,
    pub mae: f32,
    pub r2_score: f32,
    pub weights: HashMap<String, f32>,
    pub version: String,
}

impl Default for QualityModel {
    fn default() -> Self {
        Self::new()
    }
}

// Make QualityModel serializable for saving/loading
impl Serialize for QualityModel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("QualityModel", 3)?;
        state.serialize_field("samples", &self.samples)?;
        state.serialize_field("weights", &self.weights)?;
        state.serialize_field("version", &self.version)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for QualityModel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct QualityModelData {
            samples: Vec<TrainingSample>,
            weights: HashMap<String, f32>,
            version: String,
        }
        
        let data = QualityModelData::deserialize(deserializer)?;
        Ok(QualityModel {
            samples: data.samples,
            weights: data.weights,
            version: data.version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_features() {
        let code = r#"
        function test() {
            // Comment
            const payload = "test";
            if (true) {
                return payload;
            }
        }
        "#;
        
        let features = QualityModel::extract_features(code);
        assert!(features.has_comments);
        assert!(features.function_count > 0);
        assert!(features.complexity > 0.0);
    }
    
    #[test]
    fn test_model_prediction() {
        let model = QualityModel::new();
        
        let features = CodeFeatures {
            loc: 100,
            function_count: 3,
            has_comments: true,
            has_types: true,
            has_error_handling: true,
            complexity: 20.0,
            payload_count: 5,
            uses_regex: true,
        };
        
        let score = model.predict(&features).unwrap();
        assert!((0.0..=100.0).contains(&score));
    }
}

