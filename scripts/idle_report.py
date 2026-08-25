#!/usr/bin/env python3
"""rwatch の server.jsonl から OCI Always Free の idle 判定条件を評価する。

Oracle の基準（7日間の窓で以下がすべて成立すると idle 扱い）:
  - CPU使用率 (95パーセンタイル) < 20%
  - ネットワーク使用率 < 20%
  - メモリ使用率 < 20% （A1 シェイプのみ対象）

使い方:
  idle_report.py                # 直近7日を評価
  idle_report.py --hours 48    # 直近48時間を評価
"""
import argparse
import json
import os
from datetime import datetime, timedelta

DEFAULT_LOG = os.path.join(os.path.dirname(__file__), "..", "server.jsonl")
THRESHOLD_PERCENT = 20.0
DEFAULT_WINDOW_HOURS = 24 * 7


def percentile(sorted_values, p):
    """ソート済みリストの線形補間パーセンタイル。"""
    if not sorted_values:
        return None
    rank = (len(sorted_values) - 1) * p / 100.0
    lower = int(rank)
    upper = min(lower + 1, len(sorted_values) - 1)
    fraction = rank - lower
    return sorted_values[lower] * (1 - fraction) + sorted_values[upper] * fraction


def load_records(path):
    records = []
    with open(path, encoding="utf-8") as file:
        for line in file:
            line = line.strip()
            if not line:
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            try:
                timestamp = datetime.strptime(record["timestamp"], "%Y-%m-%d %H:%M:%S.%f")
            except (KeyError, ValueError):
                continue
            records.append((timestamp, record))
    records.sort(key=lambda item: item[0])
    return records


def evaluate(records, now):
    """3条件それぞれの p95 と idle 側かどうかを返す。

    ネットワークは「使用率」が計測できないため、実測 Mbps の p95 をそのまま
    閾値 20 と比較する（参考値）。メモリは available の逆数 = 使用率で判定。
    """
    cpus = sorted(record["cpu_usage"] for _, record in records)
    memory_used = sorted(100.0 - record["available_memory_percentage"] for _, record in records)
    network = sorted(
        record["network_mbps"] for _, record in records if record.get("network_mbps") is not None
    )

    cpu_p95 = percentile(cpus, 95)
    mem_p95 = percentile(memory_used, 95)
    net_p95 = percentile(network, 95)

    return {
        "cpu_p95": cpu_p95,
        "memory_p95": mem_p95,
        "network_p95_mbps": net_p95,
        "cpu_idle_side": cpu_p95 is not None and cpu_p95 < THRESHOLD_PERCENT,
        "memory_idle_side": mem_p95 is not None and mem_p95 < THRESHOLD_PERCENT,
        "network_idle_side": net_p95 is not None and net_p95 < THRESHOLD_PERCENT,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--log", default=os.environ.get("RWATCH_JSONL", DEFAULT_LOG))
    parser.add_argument("--hours", type=float, default=DEFAULT_WINDOW_HOURS)
    args = parser.parse_args()

    records = load_records(args.log)
    if not records:
        print("no valid records")
        raise SystemExit(1)

    latest_time = records[-1][0]
    cutoff = latest_time - timedelta(hours=args.hours)
    window = [(time, record) for time, record in records if time >= cutoff]

    print(f"window : last {args.hours:g}h ({window[0][0]} .. {latest_time}, n={len(window)})")

    result = evaluate(window, latest_time)

    def mark(condition):
        return "IDLE側 (<20)" if condition else "OK側 (>=20)"

    print(f"CPU      : p95 = {result['cpu_p95']:.2f}%  -> {mark(result['cpu_idle_side'])}")
    print(f"Memory   : p95 = {result['memory_p95']:.2f}%  -> {mark(result['memory_idle_side'])}")
    if result["network_p95_mbps"] is None:
        print("Network  : データなし (旧スキーマ期間のみ)")
    else:
        coverage = sum(
            1 for _, r in window if r.get("network_mbps") is not None
        ) / len(window) * 100
        print(
            f"Network  : p95 = {result['network_p95_mbps']:.3f} Mbps "
            f"(参考値・絶対速度, 計測カバー率 {coverage:.1f}%)"
        )
        print(f"           参考: {mark(result['network_idle_side'])}")

    all_measured = (
        result["cpu_idle_side"]
        and result["memory_idle_side"]
        and result["network_idle_side"] is not False
    )
    if result["network_p95_mbps"] is None:
        verdict = "ネットワーク未計測のため確定不可"
    else:
        verdict = "3条件とも idle 側 = 理論上回収対象" if all_measured else "idle 条件を外れている"
    print(f"\nverdict: {verdict}")


if __name__ == "__main__":
    main()
