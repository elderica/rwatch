#!/usr/bin/env python3
"""check_rwatch_alive() の単体テスト

実ファイル ~/projects/rwatch/server.jsonl には触れないため、
RWATCH_JSONL をテスト用一時ファイルに差し替えて実行する。

使い方: python3 watchdog_test_alive.py
"""
import json
import os
import sys
import tempfile
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import watchdog


def write_jsonl(path, timestamp):
    with open(path, "w", encoding="utf-8") as f:
        f.write(json.dumps({
            "timestamp": timestamp,
            "cpu_usage": 1.23,
            "available_memory_percentage": 85.0,
            "disk_usage": 35.0,
        }) + "\n")


def run_case(name, condition):
    mark = "PASS" if condition else "FAIL"
    print(f"[{mark}] {name}")
    return condition


def main():
    original = watchdog.RWATCH_JSONL
    failures = 0

    with tempfile.TemporaryDirectory() as tmpdir:
        test_file = os.path.join(tmpdir, "server.jsonl")
        now = time.time()

        # 正常: 最終記録が10秒前 → None
        watchdog.RWATCH_JSONL = test_file
        fresh = time.strftime("%Y-%m-%d %H:%M:%S.000", time.localtime(now - 10))
        write_jsonl(test_file, fresh)
        result = watchdog.check_rwatch_alive(now=now)
        failures += not run_case(f"fresh record (10s old) -> None (got {result})", result is None)

        # stale: 最終記録が600秒前（閾値300秒超）→ 発報
        stale = time.strftime("%Y-%m-%d %H:%M:%S.000", time.localtime(now - 600))
        write_jsonl(test_file, stale)
        result = watchdog.check_rwatch_alive(now=now)
        ok = result is not None and "600 分" in result.replace(" ", "") or (
            result is not None and "分間" in result
        )
        failures += not run_case(f"stale record (600s old) -> alert (got {result!r:.60})", ok)

        # 境界: 閾値ちょうど+0.7秒（now の小数部分のぶれを含む最悪値）→ 超過なので発報
        edge = time.strftime("%Y-%m-%d %H:%M:%S.000", time.localtime(now - 300))
        write_jsonl(test_file, edge)
        result = watchdog.check_rwatch_alive(now=now)
        failures += not run_case(
            f"threshold + fractional drift (300.7s) -> alert (got {result!r:.60})",
            result is not None and "分間" in result,
        )

        # ファイル欠損
        os.remove(test_file)
        result = watchdog.check_rwatch_alive(now=now)
        failures += not run_case(
            f"missing file -> alert (got {result!r:.50})",
            result is not None and "存在しません" in result,
        )

        # 破損行
        with open(test_file, "w") as f:
            f.write("not a json line\n")
        result = watchdog.check_rwatch_alive(now=now)
        failures += not run_case(
            f"corrupted line -> alert (got {result!r:.50})",
            result is not None and "読み取りに失敗" in result,
        )

        # 空ファイル
        open(test_file, "w").close()
        result = watchdog.check_rwatch_alive(now=now)
        failures += not run_case(
            f"empty file -> alert (got {result!r:.50})",
            result is not None and "読み取りに失敗" in result,
        )

    watchdog.RWATCH_JSONL = original

    print(f"\n{'ALL PASS' if failures == 0 else f'{failures} FAILURES'}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
