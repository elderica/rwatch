//! システムメトリクスの閾値監視。
//!
//! 旧 watchdog.py の判定ロジックを Rust に移植したもの。
//! 計測は rwatch 本体(metrics モジュール)が行い、このモジュールは
//! 「値と閾値の比較」と「異常メッセージの生成」だけを担当する。

/// 閾値(超過で異常)。watchdog.py の THRESHOLDS から disk/mem のみ採用。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    /// ディスク使用率 %
    pub disk_used_pct: f64,
    /// メモリ使用率 %
    pub mem_used_pct: f64,
}

/// watchdog.py の既定値と同一。
impl Default for Thresholds {
    fn default() -> Self {
        Self { disk_used_pct: 85.0, mem_used_pct: 90.0 }
    }
}

/// メモリ使用率の計算方式。
///
/// watchdog.py は MemTotal - MemAvailable を使用していたため、
/// 移植後も挙動を揃えるため Available ベースを既定とする。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryBasis {
    /// (total - available) / total。旧 watchdog.py と同じ。
    UsedFromAvailable,
}

/// 1 回分の計測値。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// ディスク使用率 %
    pub disk_used_pct: f64,
    /// メモリ使用率 %(basis の定義に従う)
    pub mem_used_pct: f64,
    /// メモリ計算の基準(記録用)
    pub memory_basis: MemoryBasis,
}

impl Thresholds {
    /// 計測値を評価し、超過項目の人間向け説明を返す。
    ///
    /// 正常なら空配列。複数項目が同時に超過した場合は全て返す
    /// (watchdog.py と同じ挙動)。
    pub fn violations(&self, sample: &Sample) -> Vec<String> {
        let mut findings = Vec::new();

        if sample.disk_used_pct > self.disk_used_pct {
            findings.push(format!(
                "disk usage {:.1}% exceeds threshold {:.1}%",
                sample.disk_used_pct, self.disk_used_pct
            ));
        }
        if sample.mem_used_pct > self.mem_used_pct {
            findings.push(format!(
                "memory usage {:.1}% exceeds threshold {:.1}%",
                sample.mem_used_pct, self.mem_used_pct
            ));
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_thresholds_match_watchdog_py() {
        let thresholds = Thresholds::default();
        assert_eq!(thresholds.disk_used_pct, 85.0);
        assert_eq!(thresholds.mem_used_pct, 90.0);
    }

    #[test]
    fn normal_sample_has_no_violations() {
        let sample = Sample {
            disk_used_pct: 42.0,
            mem_used_pct: 13.0,
            memory_basis: MemoryBasis::UsedFromAvailable,
        };
        assert!(Thresholds::default().violations(&sample).is_empty());
    }

    #[test]
    fn disk_over_threshold_is_reported() {
        let sample = Sample {
            disk_used_pct: 86.0,
            mem_used_pct: 13.0,
            memory_basis: MemoryBasis::UsedFromAvailable,
        };
        let violations = Thresholds::default().violations(&sample);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("disk"));
    }

    #[test]
    fn multiple_violations_are_all_reported() {
        let sample = Sample {
            disk_used_pct: 99.0,
            mem_used_pct: 95.0,
            memory_basis: MemoryBasis::UsedFromAvailable,
        };
        let violations = Thresholds::default().violations(&sample);
        assert_eq!(violations.len(), 2);
    }

    /// 境界値ちょうどは「超過」ではない(> で判定、watchdog.py 同様)。
    #[test]
    fn exactly_at_threshold_is_not_violation() {
        let sample = Sample {
            disk_used_pct: 85.0,
            mem_used_pct: 90.0,
            memory_basis: MemoryBasis::UsedFromAvailable,
        };
        assert!(Thresholds::default().violations(&sample).is_empty());
    }
}
