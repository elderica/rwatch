#!/usr/bin/env python3
"""ウォッチドッグの通信テスト用ラッパー

閾値を強制的に下げて異常を発生させ、Discord 通知の疎通を確認する。
"""
import json
import os
import subprocess
import sys
import time

script_dir = os.path.dirname(os.path.abspath(__file__))

# テスト用コード: watchdog をインポートして閾値を 0 にして check_once を実行
test_code = """
import sys
sys.path.insert(0, %r)
import time
import watchdog

# 閾値を 0 に下げて強制的に異常を発生させる
watchdog.THRESHOLDS['disk_used_pct'] = 0.0
watchdog.THRESHOLDS['mem_used_pct'] = 0.0
watchdog.THRESHOLDS['load1'] = 0.0

problems = watchdog.check_once()
lines = ["🧪 **ウォッチドッグ通信テストです**（正常稼働中）", ""]
if problems:
    lines.extend(problems)
else:
    lines.append("異常は検出されませんでした（テスト用に閾値を下げています）")
lines.append("")
lines.append("（通信テスト時刻: " + time.strftime('%%Y-%%m-%%d %%H:%%M:%%S %%Z') + "）")
print("\\n".join(lines))
""" % script_dir

result = subprocess.run(
    [sys.executable, "-c", test_code],
    capture_output=True, text=True, timeout=60,
)
sys.stdout.write(result.stdout)
if result.stderr:
    sys.stderr.write(result.stderr)
sys.exit(result.returncode)
