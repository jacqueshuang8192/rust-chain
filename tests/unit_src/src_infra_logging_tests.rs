use super::*;
use std::{
    io,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
struct Buffer(Arc<Mutex<Vec<u8>>>);
impl io::Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn logs_are_chinese_single_line_and_omit_secrets_and_request_spans() {
    let output = Arc::new(Mutex::new(Vec::new()));
    let buffer = Buffer(output.clone());
    let subscriber = tracing_subscriber::fmt()
        .event_format(ChineseLogFormat)
        .with_writer(move || buffer.clone())
        .finish();
    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("request", uri = "/ws/private?token=PRIVATE");
        let _entered = span.enter();
        tracing::info!(
            scanned = 2_u32,
            settled = 1_u32,
            elapsed_ms = 12_u64,
            payload = "CALLBACK-SECRET",
            token = "TOKEN-SECRET",
            error = "mysql://root:DB-SECRET@localhost/test\nanother line",
            "秒合约结算批次完成"
        );
    });
    let log = String::from_utf8(output.lock().unwrap().clone()).unwrap();
    assert_eq!(log.lines().count(), 1);
    for expected in [
        "[信息]",
        "秒合约结算批次完成",
        "扫描数=2",
        "结算数=1",
        "耗时毫秒=12",
        "载荷=[已隐藏]",
        "模块=",
    ] {
        assert!(log.contains(expected), "{log}");
    }
    for secret in [
        "PRIVATE",
        "CALLBACK-SECRET",
        "TOKEN-SECRET",
        "DB-SECRET",
        "\u{1b}",
    ] {
        assert!(!log.contains(secret), "{log}");
    }
}
