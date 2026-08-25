use std::fs;

const PROC_NET_DEV: &str = "/proc/net/dev";
const LOOPBACK: &str = "lo";

/// /proc/net/dev から読んだ累計バイト数（loopback 以外の全 NIC の合計）。
///
/// OCI の idle 判定（ネットワーク使用率 < 20%）の観測用。
/// VNIC のリンク速度は virtio では取得できないため、絶対値 [Mbps] のみを扱う。
#[derive(Clone, Copy, PartialEq)]
pub struct TrafficBytes {
    pub received: u64,
    pub transmitted: u64,
}

impl TrafficBytes {
    /// 現在の累計バイト数を読む。/proc/net/dev が読めない場合は None。
    pub fn read() -> Option<Self> {
        let content = fs::read_to_string(PROC_NET_DEV).ok()?;
        Self::parse(&content)
    }

    fn parse(content: &str) -> Option<Self> {
        let mut total = Self { received: 0, transmitted: 0 };

        for line in content.lines().skip(2) {
            let Some((name, stats)) = line.split_once(':') else { continue };
            if name.trim() == LOOPBACK {
                continue;
            }

            let stats: Vec<&str> = stats.split_whitespace().collect();
            // 各データ行は「受信8フィールド + 送信8フィールド」。
            // 先頭（受信バイト数）と9番目＝index 8（送信バイト数）だけを使う。
            let received: u64 = stats.first()?.parse().ok()?;
            let transmitted: u64 = stats.get(8)?.parse().ok()?;

            total.received += received;
            total.transmitted += transmitted;
        }

        Some(total)
    }

    /// 前回値との差分から区間平均スループット [Mbps] を求める。
    /// カウンタの巻き戻り（再起動など）は差分 0 として扱う。
    pub fn megabits_per_sec(previous: Self, current: Self, elapsed_seconds: f64) -> f64 {
        if elapsed_seconds <= 0.0 {
            return 0.0;
        }

        let received = current.received.saturating_sub(previous.received);
        let transmitted = current.transmitted.saturating_sub(previous.transmitted);

        (received + transmitted) as f64 * 8.0 / 1_000_000.0 / elapsed_seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo: 7292776   70629    0    0    0     0          0         0 7292776   70629    0    0    0     0       0          0
enp0s6: 1331458466 4812343    0    0    0     0          0         0 1161060552 4576496    0    0    0     0       0          0
";

    #[test]
    fn excludes_loopback_and_sums_interfaces() {
        let traffic = TrafficBytes::parse(SAMPLE).unwrap();
        assert_eq!(traffic.received, 1_331_458_466);
        assert_eq!(traffic.transmitted, 1_161_060_552);
    }

    #[test]
    fn computes_mbps_from_delta() {
        // 1 秒で 1,250,000 バイト = 10,000,000 ビット → 10 Mbps
        let previous = TrafficBytes { received: 0, transmitted: 0 };
        let current = TrafficBytes { received: 1_250_000, transmitted: 0 };
        assert!((TrafficBytes::megabits_per_sec(previous, current, 1.0) - 10.0).abs() < 1e-9);
    }

    #[test]
    fn counter_reset_does_not_go_negative() {
        let previous = TrafficBytes { received: 100, transmitted: 100 };
        let current = TrafficBytes { received: 50, transmitted: 50 };
        assert_eq!(TrafficBytes::megabits_per_sec(previous, current, 5.0), 0.0);
    }

    #[test]
    fn zero_elapsed_does_not_divide_by_zero() {
        let traffic = TrafficBytes { received: 1000, transmitted: 1000 };
        assert_eq!(TrafficBytes::megabits_per_sec(traffic, traffic, 0.0), 0.0);
    }
}
