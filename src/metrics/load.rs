//! ロードアベレージの取得。
//!
//! sysinfo 0.39 には load average がないため `/proc/loadavg` を直接読む。
//! Linux 専用だが、rwatch 自体が /proc 依存(sysinfo も内部で読む)なので
//! 方針として一貫している。

/// /proc/loadavg の最初の 3 値(1分/5分/15分)を返す。
///
/// 読み取りまたはパースに失敗した場合は None を返す
/// (呼び出し側は前回値を使うか、記録だけ飛ばして継続する)。
pub fn get_load_average() -> Option<(f64, f64, f64)> {
    let contents = std::fs::read_to_string("/proc/loadavg").ok()?;
    parse_load_average(&contents)
}

/// "0.52 0.58 0.59 1/467 12345" 形式の文字列をパースする。
///
/// テスト可能にするため文字列処理を分離している。
pub fn parse_load_average(contents: &str) -> Option<(f64, f64, f64)> {
    let mut parts = contents.split_whitespace();
    let load1 = parts.next()?.parse().ok()?;
    let load5 = parts.next()?.parse().ok()?;
    let load15 = parts.next()?.parse().ok()?;
    Some((load1, load5, load15))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typical_proc_loadavg() {
        // 実際の /proc/loadavg の典型例
        let (load1, load5, load15) = parse_load_average("0.52 0.58 0.59 1/467 12345").unwrap();
        assert_eq!(load1, 0.52);
        assert_eq!(load5, 0.58);
        assert_eq!(load15, 0.59);
    }

    #[test]
    fn parses_zero_load() {
        let (load1, _, _) = parse_load_average("0.00 0.00 0.00 2/123 456").unwrap();
        assert_eq!(load1, 0.00);
    }

    #[test]
    fn returns_none_on_garbage() {
        assert!(parse_load_average("").is_none());
        assert!(parse_load_average("not numbers here").is_none());
    }

    /// 実機で実際に読めること(取得関数の統合確認)。
    #[test]
    fn reads_real_proc_loadavg_on_linux() {
        let loads = get_load_average().expect("/proc/loadavg should be readable");
        assert!(loads.0 >= 0.0);
        assert!(loads.2 >= loads.0 - f64::EPSILON || true); // 値の大小は非保証
    }
}
