/// 测试场景编排模块
/// 支持多请求串行执行，变量传递，失败跳过
use crate::models::Response;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ScenarioStep {
    /// 请求在集合中的索引
    pub request_index: usize,
    /// 步骤间延迟毫秒
    pub delay_ms: u64,
    /// 失败时跳过后续步骤
    pub skip_on_fail: bool,
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_index: usize,
    pub request_index: usize,
    pub response: Option<Response>,
    pub skipped: bool,
    pub error: Option<String>,
}

/// 执行场景：根据步骤列表和响应列表生成结果
/// 实际的请求发送由调用方负责，这里只处理编排逻辑
pub fn plan_scenario(steps: &[ScenarioStep], total_requests: usize) -> Vec<StepResult> {
    let mut results = Vec::new();
    let mut should_skip = false;

    for (i, step) in steps.iter().enumerate() {
        if should_skip {
            results.push(StepResult {
                step_index: i,
                request_index: step.request_index,
                response: None,
                skipped: true,
                error: None,
            });
            continue;
        }

        if step.request_index >= total_requests {
            results.push(StepResult {
                step_index: i,
                request_index: step.request_index,
                response: None,
                skipped: false,
                error: Some(format!(
                    "request_index {} out of range (total: {})",
                    step.request_index, total_requests
                )),
            });
            if step.skip_on_fail {
                should_skip = true;
            }
            continue;
        }

        results.push(StepResult {
            step_index: i,
            request_index: step.request_index,
            response: None,
            skipped: false,
            error: None,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_step_default() {
        let step = ScenarioStep::default();
        assert_eq!(step.request_index, 0);
        assert_eq!(step.delay_ms, 0);
        assert!(!step.skip_on_fail);
    }

    #[test]
    fn test_scenario_step_serialization() {
        let step = ScenarioStep { request_index: 2, delay_ms: 500, skip_on_fail: true };
        let json = serde_json::to_string(&step).unwrap();
        let loaded: ScenarioStep = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.request_index, 2);
        assert!(loaded.skip_on_fail);
    }

    #[test]
    fn test_plan_scenario_empty() {
        let results = plan_scenario(&[], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_plan_scenario_invalid_index_with_skip() {
        let steps = vec![
            ScenarioStep { request_index: 0, delay_ms: 0, skip_on_fail: false },
            ScenarioStep { request_index: 99, delay_ms: 0, skip_on_fail: true },
            ScenarioStep { request_index: 1, delay_ms: 0, skip_on_fail: false },
        ];
        let results = plan_scenario(&steps, 3);
        assert_eq!(results.len(), 3);
        assert!(!results[0].skipped);
        assert!(results[1].error.is_some()); // index 99 out of range
        assert!(results[2].skipped); // skipped because previous failed with skip_on_fail
    }
}
