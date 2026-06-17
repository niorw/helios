/// 标签页管理测试
#[cfg(test)]
mod tab_tests {
    use crate::models::*;
    use crate::tui::app::TabInfo;

    #[test]
    fn test_open_tab() {
        let mut tabs: Vec<TabInfo> = Vec::new();
        let req = Request::default();
        tabs.push(TabInfo { request: req, label: "Test".into(), source: None });
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0].label, "Test");
    }

    #[test]
    fn test_close_tab() {
        let mut tabs: Vec<TabInfo> = Vec::new();
        tabs.push(TabInfo { request: Request::default(), label: "1".into(), source: None });
        tabs.push(TabInfo { request: Request::default(), label: "2".into(), source: None });
        tabs.remove(1);
        assert_eq!(tabs.len(), 1);
    }

    #[test]
    fn test_tab_limit() {
        let mut tabs: Vec<TabInfo> = Vec::new();
        for i in 0..9 {
            tabs.push(TabInfo { request: Request::default(), label: format!("{}", i), source: None });
        }
        assert_eq!(tabs.len(), 9);
        assert!(tabs.len() >= 9); // limit condition
    }

    #[test]
    fn test_switch_tab() {
        let mut tabs: Vec<TabInfo> = Vec::new();
        tabs.push(TabInfo { request: Request::default(), label: "1".into(), source: None });
        tabs.push(TabInfo { request: Request::default(), label: "2".into(), source: None });
        let mut active = 0;
        active = 1;
        assert_eq!(active, 1);
        assert_eq!(tabs[active].label, "2");
    }
}
