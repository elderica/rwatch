#!/usr/bin/env python3
"""uv venv ラッパー: VIRTUAL_ENV を明示して scripts/watchdog.py を実行する。

Hermes cron (no_agent) は $HERMES_HOME/scripts/ 配下のスクリプトしか解決できないため、
そこにはシンボリックリンクを置き、実体はこのラッパーにする。
ラッパーは自分自身の実パス(realpath)からプロジェクトルートを解決するので、
シンボリックリンク経由でも正しく動く。

依存を追加したら `uv sync` で .venv を更新する(ラッパー側の変更は不要)。
"""
import os
import subprocess
import sys

def main():
    self_real = os.path.realpath(__file__)
    project_root = os.path.dirname(os.path.dirname(self_real))   # <repo>/bin/<this> → <repo>
    target = os.path.join(project_root, "scripts", "watchdog.py")
    venv_python = os.path.join(project_root, ".venv", "bin", "python")

    if not os.path.exists(target):
        print(f"wrapper error: {target} not found")
        return 1
    if not os.path.exists(venv_python):
        print(f"wrapper error: venv not found at {venv_python}. run `uv sync` first.")
        return 1

    env = dict(os.environ)
    env["VIRTUAL_ENV"] = os.path.join(project_root, ".venv")      # 明示指定
    env["PATH"] = os.path.join(project_root, ".venv", "bin") + ":" + env.get("PATH", "")

    result = subprocess.run([venv_python, target], env=env)
    return result.returncode

if __name__ == "__main__":
    sys.exit(main())
