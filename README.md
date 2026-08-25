# rwatch

ホストのメトリクス(CPU / メモリ / ディスク / ネットワーク)を5秒間隔で収集し、
JSONL(`server.jsonl`)へ記録する常駐モニタ。

## ビルド・実行

```bash
cargo build --release
./target/release/rwatch            # 常駐(SIGTERM/SIGINT で graceful shutdown)
./target/release/rwatch --once     # 1回だけ計測して終了
```

本番運用は systemd user unit (`~/.config/systemd/user/rwatch.service`)。
デプロイ手順は DEPLOY.md を参照。

## スクリプト(scripts/)

| ファイル | 用途 |
|---|---|
| `watchdog.py` | サーバー健全性監視(disk/mem/load)+ rwatch 生死判定(server.jsonl stale 検知) |
| `watchdog_test.py` | Discord 通知の通信テスト(閾値を0にして強制発報) |
| `watchdog_test_alive.py` | ハング検知の単体テスト |
| `idle_report.py` | OCI Always Free の idle 判定基準(p95<20%)を server.jsonl から評価 |
| `rwatch_log.sh` | rwatch のスナップショット実行ラッパー(cron 用) |
| `rwatch_log_rotate.sh` | rwatch ログの月次 gzip ローテーション |
| `test-ntp-fallback.sh` | NTP フォールバックの疎通テスト |

## watchdog の実行方法(uv venv ラッパー)

cron(no_agent)からは `scripts/watchdog.py` 経由で実行する:

```bash
VIRTUAL_ENV=<repo>/.venv uv run python scripts/watchdog.py   # 直接実行する場合
```

- `pyproject.toml` は依存ゼロ(標準ライブラリのみ)だが、uv 管理 venv を用意してある
- 依存を追加するときは `pyproject.toml` に追記して `uv sync`(ラッパー側の変更は不要)
- `~/.hermes/profiles/<profile>/scripts/watchdog.py` → `scripts/watchdog.py` の
  シンボリックリンク経由でも動く(ラッパーが realpath 自己解決するため)
