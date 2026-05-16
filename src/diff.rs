/// Represents a single line in a diff output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLine {
    /// Line only present in the new (right) string.
    Add(String),
    /// Line only present in the old (left) string.
    Remove(String),
    /// Line present in both strings.
    Equal(String),
}

/// Compute a line-level diff between two strings using the LCS algorithm.
/// Returns a Vec<DiffLine> describing how to transform `a` into `b`.
pub fn diff_strings(a: &str, b: &str) -> Vec<DiffLine> {
    let a_lines: Vec<&str> = a.lines().collect();
    let b_lines: Vec<&str> = b.lines().collect();

    let lcs = compute_lcs(&a_lines, &b_lines);
    build_diff(&a_lines, &b_lines, &lcs)
}

/// Compute the Longest Common Subsequence table for two slices of lines.
fn compute_lcs<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<Vec<usize>> {
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    dp
}

/// Walk the LCS table backwards to produce the diff output.
fn build_diff(a: &[&str], b: &[&str], dp: &[Vec<usize>]) -> Vec<DiffLine> {
    let mut result = Vec::new();
    let mut i = a.len();
    let mut j = b.len();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && a[i - 1] == b[j - 1] {
            result.push(DiffLine::Equal(a[i - 1].to_string()));
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            result.push(DiffLine::Add(b[j - 1].to_string()));
            j -= 1;
        } else {
            result.push(DiffLine::Remove(a[i - 1].to_string()));
            i -= 1;
        }
    }

    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_strings() {
        let a = "hello\nworld";
        let result = diff_strings(a, a);
        assert_eq!(
            result,
            vec![
                DiffLine::Equal("hello".to_string()),
                DiffLine::Equal("world".to_string()),
            ]
        );
    }

    #[test]
    fn test_completely_different_strings() {
        let a = "aaa\nbbb";
        let b = "ccc\nddd";
        let result = diff_strings(a, b);
        assert_eq!(
            result,
            vec![
                DiffLine::Remove("aaa".to_string()),
                DiffLine::Remove("bbb".to_string()),
                DiffLine::Add("ccc".to_string()),
                DiffLine::Add("ddd".to_string()),
            ]
        );
    }

    #[test]
    fn test_additions_only() {
        let a = "line1";
        let b = "line1\nline2";
        let result = diff_strings(a, b);
        assert_eq!(
            result,
            vec![
                DiffLine::Equal("line1".to_string()),
                DiffLine::Add("line2".to_string()),
            ]
        );
    }

    #[test]
    fn test_removals_only() {
        let a = "line1\nline2\nline3";
        let b = "line1\nline3";
        let result = diff_strings(a, b);
        assert_eq!(
            result,
            vec![
                DiffLine::Equal("line1".to_string()),
                DiffLine::Remove("line2".to_string()),
                DiffLine::Equal("line3".to_string()),
            ]
        );
    }

    #[test]
    fn test_mixed_changes() {
        let a = "one\ntwo\nthree\nfour";
        let b = "one\nthree\nfour\nfive";
        let result = diff_strings(a, b);
        assert_eq!(
            result,
            vec![
                DiffLine::Equal("one".to_string()),
                DiffLine::Remove("two".to_string()),
                DiffLine::Equal("three".to_string()),
                DiffLine::Equal("four".to_string()),
                DiffLine::Add("five".to_string()),
            ]
        );
    }
}
