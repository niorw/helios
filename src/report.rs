/// 测试报告模块
/// 集合运行后生成通过率、耗时统计报告
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub duration_ms: u64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub duration_ms: u64,
    pub pass_rate: f64,
    pub results: Vec<TestResult>,
}

/// 从测试结果列表生成报告
pub fn generate_report(results: &[TestResult]) -> Report {
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = total - passed;
    let duration_ms = results.iter().map(|r| r.duration_ms).sum();
    let pass_rate = if total > 0 {
        (passed as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    Report {
        total,
        passed,
        failed,
        duration_ms,
        pass_rate,
        results: results.to_vec(),
    }
}

/// 格式化报告为终端可读字符串
pub fn format_report(report: &Report) -> String {
    let mut out = String::new();
    out.push_str(&format!("Test Report\n"));
    out.push_str(&format!("{}\n", "-".repeat(40)));
    out.push_str(&format!("Total:    {}\n", report.total));
    out.push_str(&format!("Passed:   {}\n", report.passed));
    out.push_str(&format!("Failed:   {}\n", report.failed));
    out.push_str(&format!("Duration: {}ms\n", report.duration_ms));
    out.push_str(&format!("Pass Rate: {:.1}%\n", report.pass_rate));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_counts_all() {
        let results = vec![
            TestResult { name: "A".into(), passed: true, duration_ms: 100, error_message: None },
            TestResult { name: "B".into(), passed: false, duration_ms: 200, error_message: Some("err".into()) },
            TestResult { name: "C".into(), passed: true, duration_ms: 150, error_message: None },
        ];
        let report = generate_report(&results);
        assert_eq!(report.total, 3);
    }

    #[test]
    fn test_report_passed_failed() {
        let results = vec![
            TestResult { name: "A".into(), passed: true, duration_ms: 100, error_message: None },
            TestResult { name: "B".into(), passed: false, duration_ms: 200, error_message: Some("err".into()) },
        ];
        let report = generate_report(&results);
        assert_eq!(report.passed, 1);
        assert_eq!(report.failed, 1);
    }

    #[test]
    fn test_report_duration() {
        let results = vec![
            TestResult { name: "A".into(), passed: true, duration_ms: 100, error_message: None },
            TestResult { name: "B".into(), passed: true, duration_ms: 250, error_message: None },
        ];
        let report = generate_report(&results);
        assert_eq!(report.duration_ms, 350);
    }

    #[test]
    fn test_report_empty() {
        let results = vec![];
        let report = generate_report(&results);
        assert_eq!(report.total, 0);
        assert_eq!(report.pass_rate, 0.0);
    }

    #[test]
    fn test_report_pass_rate() {
        let results = vec![
            TestResult { name: "A".into(), passed: true, duration_ms: 100, error_message: None },
            TestResult { name: "B".into(), passed: true, duration_ms: 100, error_message: None },
            TestResult { name: "C".into(), passed: false, duration_ms: 100, error_message: Some("err".into()) },
            TestResult { name: "D".into(), passed: false, duration_ms: 100, error_message: Some("err".into()) },
        ];
        let report = generate_report(&results);
        assert_eq!(report.pass_rate, 50.0);
    }

    #[test]
    fn test_format_report() {
        let results = vec![
            TestResult { name: "A".into(), passed: true, duration_ms: 100, error_message: None },
        ];
        let report = generate_report(&results);
        let formatted = format_report(&report);
        assert!(formatted.contains("Test Report"));
        assert!(formatted.contains("Passed:   1"));
    }
}
