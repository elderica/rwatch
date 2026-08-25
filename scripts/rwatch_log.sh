#!/bin/bash
# rwatch 定期実行スクリプト
# - rwatch は連続監視（5秒間隔）のため、timeout でラップしてスナップショットを取得
# - 出力は ~/logs/rwatch.log に追記
# - サイレント設計: 正常時は stdout 空 → cron no_agent で配信されない
LOG="$HOME/logs/rwatch.log"
mkdir -p "$(dirname "$LOG")"
timeout 10 "$HOME/bin/rwatch" >> "$LOG" 2>&1
exit 0