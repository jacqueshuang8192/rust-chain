use super::super::{
    MarketFeedConfig, MarketFeedGenerationFence, MarketFeedRestFallbackConfig,
    MarketFeedRestFallbackRuntime, MarketFeedSupervisorEvent, MarketFeedWorker,
    MarketFeedWriteFence, MarketIngestionSink, ReqwestMarketFeedRestFallbackHttpClient,
    run_provider_reconnect_loop_with_cancellation, run_provider_socket_with_progress,
};
use super::*;
use crate::modules::{
    events::{EventBroadcastHub, WebSocketChannel},
    market::{MarketDepthSnapshot, MarketKlineSnapshot, MarketTickerSnapshot},
};
use axum::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::sync::{Arc, Mutex};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tokio_util::sync::CancellationToken;

fn frame(time: i64) -> MarketFeedFrame {
    MarketFeedFrame::bitget_ticker(
        json!({
            "arg":{"channel":"ticker","instId":"BTCUSDT"}, "ts":time,
            "data":[{"lastPr":"100", "baseVolume":"1", "ts":time.to_string()}]
        })
        .to_string(),
    )
}

#[test]
fn busy_stale_frames_eventually_expire_despite_continued_activity() {
    let mut progress = SourceProgress::default();
    let wall = Utc::now();
    let clock = Instant::now();
    for second in 0..10 {
        // The wire remains busy while its source clock only advances half as fast.
        let old = frame(wall.timestamp_millis() - 450_000 + second * 500);
        assert!(
            progress
                .check(
                    &old,
                    wall + chrono::Duration::seconds(second),
                    clock + Duration::from_secs(second as u64)
                )
                .is_ok()
        );
    }
    assert!(
        progress
            .check(
                &frame(wall.timestamp_millis() - 445_000),
                wall + chrono::Duration::seconds(10),
                clock + Duration::from_secs(10)
            )
            .unwrap_err()
            .to_string()
            .contains("source backlog")
    );
}

#[test]
fn fresh_push_resets_lag_grace_and_transient_bursts_do_not_disconnect() {
    let mut progress = SourceProgress::default();
    let wall = Utc::now();
    let clock = Instant::now();
    let old = frame(wall.timestamp_millis() - 100_000);
    progress.check(&old, wall, clock).unwrap();
    progress
        .check(&old, wall, clock + Duration::from_secs(9))
        .unwrap();
    progress
        .check(
            &frame(wall.timestamp_millis()),
            wall,
            clock + Duration::from_secs(10),
        )
        .unwrap();
    progress
        .check(&old, wall, clock + Duration::from_secs(11))
        .unwrap();
    progress
        .check(&old, wall, clock + Duration::from_secs(12))
        .unwrap();
    assert!(
        progress
            .check(&old, wall, clock + Duration::from_secs(20))
            .is_ok()
    );
    assert!(
        progress
            .check(&old, wall, clock + Duration::from_secs(21))
            .is_err()
    );
}

#[test]
fn requires_three_frames_and_monotonic_grace_not_a_single_historical_snapshot() {
    let mut progress = SourceProgress::default();
    let wall = Utc::now();
    let clock = Instant::now();
    let old = frame(wall.timestamp_millis() - 100_000);
    progress.check(&old, wall, clock).unwrap();
    progress
        .check(&old, wall, clock + Duration::from_secs(20))
        .unwrap();
    assert!(
        progress
            .check(&old, wall, clock + Duration::from_secs(21))
            .is_err()
    );
    let mut progress = SourceProgress::default();
    for _ in 0..20 {
        progress
            .check(&old, wall + chrono::Duration::hours(1), clock)
            .unwrap();
    }
}

#[test]
fn envelope_time_never_uses_historical_candle_trade_or_ticker_record_time() {
    for payload in [
        json!({"data":[["1700000000000","1","2","1","2","1"]]}),
        json!({"data":[{"ts":"1700000000000"}]}),
        json!({"ping":1_700_000_000_000_i64}),
        json!({"ts":null}),
        json!({"ts":0}),
        json!({"ts":1.5}),
        json!({"ts":"NaN"}),
        json!({"ts":i64::MAX}),
    ] {
        assert!(envelope_time(&MarketFeedFrame::bitget_kline(payload.to_string())).is_none());
    }
    let wall = Utc::now();
    let recent = MarketFeedFrame::bitget_kline(
        json!({"ts":wall.timestamp_millis(),"data":[["1700000000000"]]}).to_string(),
    );
    assert_eq!(
        envelope_time(&recent).unwrap().timestamp_millis(),
        wall.timestamp_millis()
    );
    let mut progress = SourceProgress::default();
    let clock = Instant::now();
    for i in 0..20 {
        progress
            .check(&recent, wall, clock + Duration::from_secs(i))
            .unwrap();
    }
}

#[test]
fn provider_envelopes_accept_milliseconds_and_rfc3339_without_touching_payloads() {
    let time = 1_790_000_000_123;
    assert_eq!(
        envelope_time(&frame(time)).unwrap().timestamp_millis(),
        time
    );
    let htx = MarketFeedFrame::htx_depth(json!({"ts":time.to_string()}).to_string());
    assert_eq!(envelope_time(&htx).unwrap().timestamp_millis(), time);
    let instant = DateTime::from_timestamp_millis(time).unwrap();
    let coinbase =
        MarketFeedFrame::coinbase_ticker(json!({"timestamp":instant.to_rfc3339()}).to_string());
    assert_eq!(envelope_time(&coinbase), Some(instant));
}

#[test]
fn missing_timestamp_does_not_erase_detected_lag_and_future_time_does_not_reconnect() {
    let wall = Utc::now();
    let clock = Instant::now();
    let old = frame(wall.timestamp_millis() - 100_000);
    let mut progress = SourceProgress::default();
    progress.check(&old, wall, clock).unwrap();
    progress
        .check(&old, wall, clock + Duration::from_secs(1))
        .unwrap();
    progress
        .check(
            &MarketFeedFrame::bitget_ticker("{}"),
            wall,
            clock + Duration::from_secs(9),
        )
        .unwrap();
    assert!(
        progress
            .check(&old, wall, clock + Duration::from_secs(10))
            .is_err()
    );
    let mut progress = SourceProgress::default();
    for i in 0..20 {
        progress
            .check(
                &frame(wall.timestamp_millis() + 100_000),
                wall,
                clock + Duration::from_secs(i),
            )
            .unwrap();
    }
}

#[derive(Clone, Default)]
struct RecordingSink(Arc<Mutex<Vec<i64>>>);

#[async_trait]
impl MarketIngestionSink for RecordingSink {
    async fn ingest_ticker(&self, snapshot: &MarketTickerSnapshot) -> AppResult<()> {
        self.0
            .lock()
            .unwrap()
            .push(snapshot.observed_at().timestamp_millis());
        Ok(())
    }
    async fn ingest_depth(&self, _: &MarketDepthSnapshot) -> AppResult<()> {
        Ok(())
    }
    async fn ingest_kline(&self, _: &MarketKlineSnapshot) -> AppResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn actual_socket_cycle_reconnects_busy_stale_source_and_publishes_fresh_data() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let old = Utc::now().timestamp_millis() - 450_000;
    let fresh = Utc::now().timestamp_millis();
    let server = tokio::spawn(async move {
        for times in [vec![old, old + 1, old + 2], vec![fresh]] {
            let (socket, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(socket).await.unwrap();
            assert_eq!(
                socket.next().await.unwrap().unwrap(),
                Message::Text("subscribe-test".into())
            );
            for time in times {
                socket.send(Message::Text("pong".into())).await.unwrap();
                socket
                    .send(Message::Text(frame(time).payload().into()))
                    .await
                    .unwrap();
            }
            let _ = socket.close(None).await;
        }
    });
    let sink = RecordingSink::default();
    let hub = EventBroadcastHub::new(16);
    let mut messages = hub.subscribe(&WebSocketChannel::public("ticker", "BTCUSDT").unwrap());
    let worker = MarketFeedWorker::new(sink.clone()).with_broadcast_hub(hub);
    let fallback_worker = worker.clone();
    let fence = MarketFeedGenerationFence::default();
    fence.activate(1).await;
    let stop = CancellationToken::new();
    let socket_stop = stop.clone();
    let event_stop = stop.clone();
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = events.clone();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        run_provider_reconnect_loop_with_cancellation(
            crate::state::AppState::new(super::super::tests::test_settings()),
            MarketFeedConfig::new(
                MarketFeedProvider::Bitget,
                format!("ws://{address}"),
                vec!["subscribe-test".into()],
                vec!["BTCUSDT".into()],
                vec!["1m".into()],
            ),
            Duration::from_millis(1),
            move |_, config| {
                run_provider_socket_with_progress(
                    worker.clone(),
                    config,
                    MarketFeedWriteFence::new(1, fence.clone()),
                    socket_stop.clone(),
                    SourceProgress::new(Duration::from_secs(30), Duration::ZERO),
                )
            },
            MarketFeedRestFallbackRuntime::new(
                MarketFeedRestFallbackConfig::new(MarketFeedProvider::Bitget, vec![], vec![]),
                move |_| {
                    let worker = fallback_worker.clone();
                    async move { Ok(worker) }
                },
                ReqwestMarketFeedRestFallbackHttpClient::default(),
            ),
            move |event| {
                if matches!(
                    event,
                    MarketFeedSupervisorEvent::ProviderCycleSucceeded { .. }
                ) {
                    event_stop.cancel();
                }
                captured.lock().unwrap().push(event);
            },
            stop,
        ),
    )
    .await
    .unwrap();
    result.unwrap();
    tokio::time::timeout(Duration::from_secs(1), server)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(*sink.0.lock().unwrap(), vec![old, old + 1, old + 2, fresh]);
    {
        let events = events.lock().unwrap();
        assert!(
            matches!(&events[0], MarketFeedSupervisorEvent::ProviderCycleFailed { error, .. } if error.contains("source backlog"))
        );
        assert!(matches!(
            &events[1],
            MarketFeedSupervisorEvent::ProviderCycleSucceeded { .. }
        ));
    }
    for _ in 0..4 {
        tokio::time::timeout(Duration::from_secs(1), messages.recv())
            .await
            .unwrap()
            .unwrap();
    }
}
