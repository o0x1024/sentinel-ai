//! 综合压力测试运行器和报告生成
//!
//! 提供统一的测试运行和报告生成工具

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::time::Duration;

/// 测试类别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestCategory {
    Memory,
    Cpu,
    Concurrency,
    V8Limits,
    Integration,
}

/// 测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_name: String,
    pub category: TestCategory,
    pub passed: bool,
    pub duration: Duration,
    pub iterations: usize,
    pub success_count: usize,
    pub error_count: usize,
    pub peak_memory_mb: f64,
    pub avg_memory_mb: f64,
    pub peak_cpu_percent: f64,
    pub avg_cpu_percent: f64,
    pub throughput_per_sec: f64,
    pub error_messages: Vec<String>,
    pub warnings: Vec<String>,
}

/// 测试套件报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuiteReport {
    pub timestamp: String,
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub total_duration: Duration,
    pub system_info: SystemInfo,
    pub results: Vec<TestResult>,
    pub recommendations: Vec<String>,
}

/// 系统信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub cpu_count: usize,
    pub total_memory_mb: f64,
}

impl TestSuiteReport {
    pub fn new() -> Self {
        use sysinfo::System;
        
        let mut sys = System::new_all();
        sys.refresh_all();
        
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            total_duration: Duration::from_secs(0),
            system_info: SystemInfo {
                os: std::env::consts::OS.to_string(),
                arch: std::env::consts::ARCH.to_string(),
                cpu_count: num_cpus::get(),
                total_memory_mb: sys.total_memory() as f64 / 1024.0,
            },
            results: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    pub fn add_result(&mut self, result: TestResult) {
        self.total_tests += 1;
        if result.passed {
            self.passed_tests += 1;
        } else {
            self.failed_tests += 1;
        }
        self.total_duration += result.duration;
        self.results.push(result);
    }

    pub fn generate_recommendations(&mut self) {
        // 基于测试结果生成建议
        
        // 内存相关建议
        let max_memory = self.results.iter()
            .map(|r| r.peak_memory_mb)
            .fold(0.0f64, f64::max);
        
        if max_memory > self.system_info.total_memory_mb * 0.8 {
            self.recommendations.push(
                format!("⚠️  Peak memory usage ({:.2} MB) exceeds 80% of system memory. Consider increasing system memory or reducing plugin concurrency.", max_memory)
            );
        }

        // CPU相关建议
        let max_cpu = self.results.iter()
            .map(|r| r.peak_cpu_percent)
            .fold(0.0f64, f64::max);
        
        if max_cpu > 90.0 {
            self.recommendations.push(
                format!("⚠️  Peak CPU usage ({:.2}%) is very high. Consider optimizing plugin code or limiting concurrent executions.", max_cpu)
            );
        }

        // 错误率建议
        let high_error_tests: Vec<_> = self.results.iter()
            .filter(|r| {
                let error_rate = r.error_count as f64 / (r.success_count + r.error_count) as f64;
                error_rate > 0.1
            })
            .collect();
        
        if !high_error_tests.is_empty() {
            self.recommendations.push(
                format!("⚠️  {} test(s) have error rate > 10%. Review error messages for details.", high_error_tests.len())
            );
        }

        // 并发建议
        let concurrency_tests: Vec<_> = self.results.iter()
            .filter(|r| matches!(r.category, TestCategory::Concurrency))
            .collect();
        
        if let Some(best_concurrency) = concurrency_tests.iter()
            .max_by(|a, b| a.throughput_per_sec.partial_cmp(&b.throughput_per_sec).unwrap())
        {
            self.recommendations.push(
                format!("✓ Optimal concurrency configuration: {} (throughput: {:.2} ops/sec)", 
                    best_concurrency.test_name, best_concurrency.throughput_per_sec)
            );
        }

        // 性能建议
        let avg_throughput = self.results.iter()
            .map(|r| r.throughput_per_sec)
            .sum::<f64>() / self.results.len() as f64;
        
        if avg_throughput < 10.0 {
            self.recommendations.push(
                "⚠️  Average throughput is low. Consider optimizing plugin code or using PluginExecutor for better performance.".to_string()
            );
        }
    }

    /// 生成 Markdown 报告
    pub fn generate_markdown_report(&self) -> String {
        let mut report = String::new();
        
        report.push_str("# Plugin System Stress Test Report\n\n");
        
        // 概览
        report.push_str("## Overview\n\n");
        report.push_str(&format!("- **Timestamp**: {}\n", self.timestamp));
        report.push_str(&format!("- **Total Tests**: {}\n", self.total_tests));
        report.push_str(&format!("- **Passed**: {} ✓\n", self.passed_tests));
        report.push_str(&format!("- **Failed**: {} ✗\n", self.failed_tests));
        report.push_str(&format!("- **Total Duration**: {:?}\n", self.total_duration));
        report.push_str(&format!("- **Success Rate**: {:.2}%\n\n", 
            self.passed_tests as f64 / self.total_tests as f64 * 100.0));
        
        // 系统信息
        report.push_str("## System Information\n\n");
        report.push_str(&format!("- **OS**: {}\n", self.system_info.os));
        report.push_str(&format!("- **Architecture**: {}\n", self.system_info.arch));
        report.push_str(&format!("- **CPU Cores**: {}\n", self.system_info.cpu_count));
        report.push_str(&format!("- **Total Memory**: {:.2} MB\n\n", self.system_info.total_memory_mb));
        
        // 测试结果按类别分组
        report.push_str("## Test Results by Category\n\n");
        
        for category in &[
            TestCategory::Memory,
            TestCategory::Cpu,
            TestCategory::Concurrency,
            TestCategory::V8Limits,
            TestCategory::Integration,
        ] {
            let category_results: Vec<_> = self.results.iter()
                .filter(|r| std::mem::discriminant(&r.category) == std::mem::discriminant(category))
                .collect();
            
            if category_results.is_empty() {
                continue;
            }
            
            report.push_str(&format!("### {:?} Tests\n\n", category));
            report.push_str("| Test Name | Status | Duration | Throughput | Peak Memory | Peak CPU | Error Rate |\n");
            report.push_str("|-----------|--------|----------|------------|-------------|----------|------------|\n");
            
            for result in category_results {
                let status = if result.passed { "✓" } else { "✗" };
                let error_rate = if result.success_count + result.error_count > 0 {
                    result.error_count as f64 / (result.success_count + result.error_count) as f64 * 100.0
                } else {
                    0.0
                };
                
                report.push_str(&format!(
                    "| {} | {} | {:?} | {:.2} ops/s | {:.2} MB | {:.2}% | {:.2}% |\n",
                    result.test_name,
                    status,
                    result.duration,
                    result.throughput_per_sec,
                    result.peak_memory_mb,
                    result.peak_cpu_percent,
                    error_rate
                ));
            }
            
            report.push_str("\n");
        }
        
        // 建议
        if !self.recommendations.is_empty() {
            report.push_str("## Recommendations\n\n");
            for rec in &self.recommendations {
                report.push_str(&format!("- {}\n", rec));
            }
            report.push_str("\n");
        }
        
        // 详细错误信息
        let failed_tests: Vec<_> = self.results.iter()
            .filter(|r| !r.passed || !r.error_messages.is_empty())
            .collect();
        
        if !failed_tests.is_empty() {
            report.push_str("## Detailed Error Messages\n\n");
            for result in failed_tests {
                report.push_str(&format!("### {}\n\n", result.test_name));
                
                if !result.error_messages.is_empty() {
                    report.push_str("**Errors:**\n\n");
                    for msg in &result.error_messages {
                        report.push_str(&format!("- {}\n", msg));
                    }
                    report.push_str("\n");
                }
                
                if !result.warnings.is_empty() {
                    report.push_str("**Warnings:**\n\n");
                    for msg in &result.warnings {
                        report.push_str(&format!("- {}\n", msg));
                    }
                    report.push_str("\n");
                }
            }
        }
        
        report
    }

    /// 生成 JSON 报告
    pub fn generate_json_report(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// 保存报告到文件
    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let markdown = self.generate_markdown_report();
        let mut file = File::create(path)?;
        file.write_all(markdown.as_bytes())?;
        Ok(())
    }

    /// 保存 JSON 报告到文件
    pub fn save_json_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = self.generate_json_report()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// 打印控制台报告
    pub fn print_console_report(&self) {
        println!("\n{}", "=".repeat(100));
        println!("PLUGIN SYSTEM STRESS TEST REPORT");
        println!("{}\n", "=".repeat(100));
        
        println!("OVERVIEW");
        println!("  Timestamp: {}", self.timestamp);
        println!("  Total Tests: {}", self.total_tests);
        println!("  Passed: {} ✓ | Failed: {} ✗", self.passed_tests, self.failed_tests);
        println!("  Success Rate: {:.2}%", self.passed_tests as f64 / self.total_tests as f64 * 100.0);
        println!("  Total Duration: {:?}\n", self.total_duration);
        
        println!("SYSTEM INFO");
        println!("  OS: {} ({})", self.system_info.os, self.system_info.arch);
        println!("  CPU Cores: {}", self.system_info.cpu_count);
        println!("  Total Memory: {:.2} MB\n", self.system_info.total_memory_mb);
        
        println!("TEST SUMMARY");
        println!("  {:<40} {:<10} {:<15} {:<15}", "Test Name", "Status", "Duration", "Throughput");
        println!("  {}", "-".repeat(80));
        
        for result in &self.results {
            let status = if result.passed { "PASS ✓" } else { "FAIL ✗" };
            println!(
                "  {:<40} {:<10} {:<15?} {:<15.2}",
                &result.test_name[..result.test_name.len().min(40)],
                status,
                result.duration,
                result.throughput_per_sec
            );
        }
        
        println!();
        
        if !self.recommendations.is_empty() {
            println!("RECOMMENDATIONS");
            for rec in &self.recommendations {
                println!("  {}", rec);
            }
            println!();
        }
        
        println!("{}\n", "=".repeat(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_generation() {
        let mut report = TestSuiteReport::new();
        
        let result = TestResult {
            test_name: "test_example".to_string(),
            category: TestCategory::Memory,
            passed: true,
            duration: Duration::from_secs(10),
            iterations: 1000,
            success_count: 950,
            error_count: 50,
            peak_memory_mb: 512.0,
            avg_memory_mb: 256.0,
            peak_cpu_percent: 75.0,
            avg_cpu_percent: 50.0,
            throughput_per_sec: 100.0,
            error_messages: vec![],
            warnings: vec![],
        };
        
        report.add_result(result);
        report.generate_recommendations();
        
        assert_eq!(report.total_tests, 1);
        assert_eq!(report.passed_tests, 1);
        
        let markdown = report.generate_markdown_report();
        assert!(markdown.contains("Plugin System Stress Test Report"));
        
        let json = report.generate_json_report().unwrap();
        assert!(json.contains("test_example"));
    }
}

