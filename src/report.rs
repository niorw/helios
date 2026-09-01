use serde::{Deserialize, Serialize};

/// Result of a single test execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub duration_ms: u64,
}

/// Aggregated report from a batch of test results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Report {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub duration_ms: u64,
}

/// Generate an aggregated report from a slice of test results.
pub fn generate_report(results: &[TestResult]) -> Report {
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = total - passed;
    let duration_ms = results.iter().map(|r| r.duration_ms).sum();
    Report {
        total,
        passed,
        failed,
        duration_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_counts_all_results() {
        let results = vec![
            TestResult { name: "test_a".into(), passed: true, duration_ms: 100 },
            TestResult { name: "test_b".into(), passed: false, duration_ms: 200 },
            TestResult { name: "test_c".into(), passed: true, duration_ms: 150 },
        ];
        let report = generate_report(&results);
        assert_eq!(report.total, 3);
    }

    #[test]
    fn test_report_passed_and_failed_counts() {
        let results = vec![
            TestResult { name: "test_a".into(), passed: true, duration_ms: 100 },
            TestResult { name: "test_b".into(), passed: false, duration_ms: 200 },
            TestResult { name: "test_c".into(), passed: true, duration_ms: 150 },
            TestResult { name: "test_d".into(), passed: false, duration_ms: 50 },
        ];
        let report = generate_report(&results);
        assert_eq!(report.passed, 2);
        assert_eq!(report.failed, 2);
    }

    #[test]
    fn test_report_sums_duration() {
        let results = vec![
            TestResult { name: "test_a".into(), passed: true, duration_ms: 100 },
            TestResult { name: "test_b".into(), passed: true, duration_ms: 250 },
            TestResult { name: "test_c".into(), passed: false, duration_ms: 50 },
        ];
        let report = generate_report(&results);
        assert_eq!(report.duration_ms, 400);
    }

    #[test]
    fn test_report_empty_results() {
        let results: Vec<TestResult> = vec![];
        let report = generate_report(&results);
        assert_eq!(report.total, 0);
        assert_eq!(report.passed, 0);
        assert_eq!(report.failed, 0);
        assert_eq!(report.duration_ms, 0);
    }
}
