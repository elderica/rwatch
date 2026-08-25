#!/usr/bin/env python3
"""サーバー健全性ウォッチドッグ

- 平常時: ログファイルに記録のみ（stdout に出さない = Discord 通知なし）
- 異常時: stdout に報告を出す（cron が Discord へ通知）

使い方:
  watchdog.py            # チェック実行（異常時のみ出力）
  watchdog.py --init     # 基準値を記録（初回のみ）
  watchdog.py --status   # 最新ログを表示
"""
import json
import os
import shutil
import sys
import time

STATE_FILE = os.path.join(
    os.path.expanduser("~"),
    ".hermes", "profiles", "shizuku-sandbox",
    "scripts", ".watchdog_state.json",
)
LOG_FILE = os.path.join(
    os.path.expanduser("~"),
    ".hermes", "profiles", "shizuku-sandbox",
    "scripts", "watchdog.log",
)

# rwatch の出力ログ（リポジトリ直下。systemd unit の WorkingDirectory と同じ場所）
RWATCH_JSONL = os.path.join(os.path.expanduser("~"), "projects", "rwatch", "server.jsonl")

# 閾値（超過で警告）
THRESHOLDS = {
    "disk_used_pct": 85.0,    # ディスク使用率 %
    "mem_used_pct": 90.0,     # メモリ使用率 %
    "load1": 4.0,             # ロードアベレージ（1分）
}

# rwatch ハング検知: server.jsonl の最終タイムスタンプがこの秒数より古ければ異常。
# 計測間隔は5秒なので、通常なら数秒の遅れしか生じない。
# NTP再同期待ちや一時的な高負荷を誤検知しないため、間隔の60倍を余裕として取る。
RWATCH_STALE_SECONDS = 300


def get_disk_usage():
    """/ の使用率を取得"""
    usage = shutil.disk_usage("/")
    pct = usage.used / usage.total * 100
    return pct, usage.total / (1024**3), usage.free / (1024**3)


def get_mem_usage():
    """メモリ使用率を取得"""
    with open("/proc/meminfo") as f:
        info = {}
        for line in f:
            k, _, v = line.partition(":")
            info[k] = int(v.strip().split()[0])  # kB
    total = info["MemTotal"]
    available = info["MemAvailable"]
    used = total - available
    return used / total * 100, total / (1024**2), available / (1024**2)


def get_load():
    """ロードアベレージ（1分）を取得"""
    with open("/proc/loadavg") as f:
        parts = f.read().split()
    return float(parts[0]), float(parts[1]), float(parts[2])


def check_rwatch_alive(now=None):
    """rwatch の生死を server.jsonl の最終タイムスタンプで判定する。

    戻り値: 正常なら None、異常なら人間向け説明文字列。

    検知できる故障:
      - プロセス停止（systemd が再起動した場合も途切れた期間として残る）
      - ハング（デッドロック等で生きているが書けない状態。systemd は検知できない）
      - ディスクフル等で書き込みに失敗し続けている状態
    """
    if now is None:
        now = time.time()

    try:
        with open(RWATCH_JSONL, "rb") as f:
            f.seek(max(-512, -os.path.getsize(RWATCH_JSONL)), os.SEEK_END)
            last_line = f.read().splitlines()[-1]
        record = json.loads(last_line.decode("utf-8"))
        last_timestamp = record["timestamp"]
    except FileNotFoundError:
        return f"server.jsonl が存在しません（{RWATCH_JSONL}）"
    except (OSError, IndexError, KeyError, UnicodeDecodeError, json.JSONDecodeError) as error:
        return f"server.jsonl の読み取りに失敗: {error}"

    last_epoch = time.mktime(time.strptime(last_timestamp[:19], "%Y-%m-%d %H:%M:%S"))
    age = now - last_epoch

    if age > RWATCH_STALE_SECONDS:
        minutes = int(age // 60)
        return (
            f"rwatch が {minutes} 分間記録を停止しています"
            f"（最終記録: {last_timestamp}、許容 {RWATCH_STALE_SECONDS} 秒）\n"
            f"  - プロセス状態: systemctl --user status rwatch.service"
        )
    return None


def log_write(entry: str):
    """ログファイルに追記（時刻付き）"""
    os.makedirs(os.path.dirname(LOG_FILE), exist_ok=True)
    ts = time.strftime("%Y-%m-%d %H:%M:%S %Z")
    with open(LOG_FILE, "a", encoding="utf-8") as f:
        f.write(f"[{ts}] {entry}\n")


def check_once():
    """1回チェック。常にログを記録し、異常があれば問題リストを返す"""
    disk_pct, disk_total, disk_free = get_disk_usage()
    mem_pct, mem_total, mem_free = get_mem_usage()
    load1, load5, load15 = get_load()

    # 平常時もログに記録（1日1回程度ならノイズにならない）
    log_write(
        f"disk={disk_pct:.1f}% mem={mem_pct:.1f}% "
        f"load={load1:.2f}/{load5:.2f}/{load15:.2f}"
    )

    problems = []
    if disk_pct > THRESHOLDS["disk_used_pct"]:
        problems.append(
            f"💾 **ディスク使用率 {disk_pct:.1f}%**（閾値 {THRESHOLDS['disk_used_pct']}% 超過）\n"
            f"  - 全体 {disk_total:.0f} GB / 空き {disk_free:.1f} GB"
        )
    if mem_pct > THRESHOLDS["mem_used_pct"]:
        problems.append(
            f"🧠 **メモリ使用率 {mem_pct:.1f}%**（閾値 {THRESHOLDS['mem_used_pct']}% 超過）\n"
            f"  - 全体 {mem_total:.0f} GB / 空き {mem_free:.1f} GB"
        )
    if load1 > THRESHOLDS["load1"]:
        problems.append(
            f"⚡ **ロードアベレージ {load1:.2f}**（閾値 {THRESHOLDS['load1']} 超過）\n"
            f"  - 1分 {load1:.2f} / 5分 {load5:.2f} / 15分 {load15:.2f}"
        )

    rwatch_problem = check_rwatch_alive()
    if rwatch_problem:
        problems.append(f"📡 **{rwatch_problem}")

    return problems


def main():
    if "--init" in sys.argv:
        disk_pct, disk_total, disk_free = get_disk_usage()
        mem_pct, mem_total, mem_free = get_mem_usage()
        load1, load5, load15 = get_load()
        state = {
            "init_time": time.strftime("%Y-%m-%d %H:%M:%S %Z"),
            "disk": f"{disk_pct:.1f}%",
            "mem": f"{mem_pct:.1f}%",
            "load": f"{load1:.2f}",
        }
        os.makedirs(os.path.dirname(STATE_FILE), exist_ok=True)
        with open(STATE_FILE, "w") as f:
            json.dump(state, f, indent=2)
        print(f"✅ 基準値を記録: disk {state['disk']} / mem {state['mem']} / load {state['load']}")
        return

    if "--status" in sys.argv:
        try:
            with open(LOG_FILE, encoding="utf-8") as f:
                lines = f.readlines()
            print("".join(lines[-20:]))
        except FileNotFoundError:
            print("ログがまだありません。")
        return

    problems = check_once()
    if not problems:
        return  # 異常なし → ログのみ記録済み、stdout は空

    lines = ["🚨 **サーバー異常を検知**", ""]
    lines.extend(problems)
    lines.append("")
    lines.append(f"（チェック時刻: {time.strftime('%Y-%m-%d %H:%M:%S %Z')}）")
    print("\n".join(lines))


if __name__ == "__main__":
    main()
