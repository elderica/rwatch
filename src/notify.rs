//! systemd の sd_notify プロトコルによる生存報告。
//!
//! unit に `Type=notify` と `WatchdogSec=` を設定すると、systemd は
//! `$NOTIFY_SOCKET` 環境変数に AF_UNIX ソケットのアドレスを渡してくる。
//! 本モジュールはそこへ `WATCHDOG=1` を送るだけの最小実装。
//!
//! 送信先が存在しない環境(手動実行、cron、unit 未設定)では何もせず
//! `Ok(false)` を返す。本体ループを壊さないための設計。

use std::ffi::OsStr;
use std::io;
use std::os::linux::net::SocketAddrExt;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::{SocketAddr, UnixDatagram};

/// `$NOTIFY_SOCKET` を読んで `message` を送る。
///
/// 戻り値:
/// - `Ok(true)`  — 送信成功(systemd 配下で watchdog 有効)
/// - `Ok(false)` — NOTIFY_SOCKET 未設定(配下でない。何もしない)
/// - `Err(..)`   — ソケット生成または送信失敗
pub fn notify(message: &str) -> io::Result<bool> {
    match std::env::var_os("NOTIFY_SOCKET") {
        Some(address) => notify_via(&address, message),
        None => Ok(false),
    }
}

/// 指定されたアドレス(`$NOTIFY_SOCKET` の値と同じ形式)へ送る。
///
/// テストから直接叩せるため env 読みと分離している。
/// アドレス形式は 2 種類:
/// - 通常パス: `/run/user/1002/systemd/notify`
/// - abstract: `@` 始まり(Linux abstract namespace、実体は NUL 始まりアドレス)
pub fn notify_via(address: &OsStr, message: &str) -> io::Result<bool> {
    let raw = address.as_bytes();

    let socket = UnixDatagram::unbound()?;
    if raw.first() == Some(&b'@') {
        let name = &raw[1..];
        let target = SocketAddr::from_abstract_name(name)?;
        socket.send_to_addr(message.as_bytes(), &target)?;
    } else {
        socket.send_to(message.as_bytes(), std::path::Path::new(address))?;
    }
    Ok(true)
}

/// systemd watchdog への ping。測定ループの末尾で呼ぶ想定。
///
/// JSONL 書き込みの**成功後**に呼ぶことで「生きているが書けない」
/// 状態も検知できる(書けなければ ping も止まり、systemd が kill する)。
pub fn watchdog_ping() -> io::Result<bool> {
    notify("WATCHDOG=1")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 通常パスソケットへの送信が成功し、受信側で内容が一致する。
    #[test]
    fn sends_message_over_path_socket() {
        let dir = std::env::temp_dir().join(format!("rwatch-notify-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sock_path = dir.join("notify.sock");
        let _ = std::fs::remove_file(&sock_path);

        let receiver = UnixDatagram::bind(&sock_path).unwrap();

        assert!(notify_via(OsStr::new(sock_path.to_str().unwrap()), "WATCHDOG=1").unwrap());

        let mut buffer = [0u8; 64];
        let (length, _) = receiver.recv_from(&mut buffer).unwrap();
        assert_eq!(&buffer[..length], b"WATCHDOG=1");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// abstract アドレス(@ 始まり)への送信が成功する。
    #[test]
    fn sends_message_over_abstract_socket() {
        // 他テストとの衝突を避けるため PID を含める
        let name = format!("rwatch-test-{}", std::process::id());
        let receiver_address = SocketAddr::from_abstract_name(name.as_bytes()).unwrap();
        let receiver = UnixDatagram::bind_addr(&receiver_address).unwrap();

        assert!(notify_via(OsStr::new(&format!("@{name}")), "WATCHDOG=1").unwrap());

        let mut buffer = [0u8; 64];
        let (length, _) = receiver.recv_from(&mut buffer).unwrap();
        assert_eq!(&buffer[..length], b"WATCHDOG=1");
    }

    /// 存在しないパスはエラーになる(黙って成功しない)。
    #[test]
    fn errors_on_missing_socket() {
        let result = notify_via(OsStr::new("/nonexistent/rwatch-notify.sock"), "WATCHDOG=1");
        assert!(result.is_err());
    }

    /// 空文字列アドレスは abstract 名として不正でエラーになる。
    #[test]
    fn errors_on_empty_abstract_name() {
        assert!(notify_via(OsStr::new("@"), "WATCHDOG=1").is_err());
    }
}
