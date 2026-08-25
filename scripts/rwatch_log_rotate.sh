#!/bin/bash
# rwatch ログのローテーション・圧縮
# - ログが 1MB を超えたら gzip して月別アーカイブに移動
# - 6ヶ月より古いアーカイブは自動削除
# - サイレント設計: 何もしないときは stdout 空
LOG="$HOME/logs/rwatch.log"
ARCHIVE_DIR="$HOME/logs"

# ログが 1MB 未満なら何もしない
if [ ! -f "$LOG" ] || [ "$(stat -c%s "$LOG")" -lt 1048576 ]; then
    exit 0
fi

# 圧縮して月別にリネーム（例: rwatch.log.202608.gz）
gzip "$LOG"
mv "$LOG.gz" "$ARCHIVE_DIR/rwatch.log.$(date +%Y%m).gz"

# 6ヶ月より古いアーカイブを削除
find "$ARCHIVE_DIR" -name "rwatch.log.*.gz" -mtime +180 -delete

exit 0