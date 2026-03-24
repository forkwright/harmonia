/// Integration tests for syndesis QUIC streaming protocol.
use std::net::SocketAddr;
use std::time::Duration;

use bytes::Bytes;
use syndesis::client::StreamClient;
use syndesis::protocol::AudioCodec;
use syndesis::protocol::frame::AudioFrame;
use syndesis::server::StreamServer;
use syndesis::server::source::VecAudioSource;

fn test_frames(count: usize) -> Vec<AudioFrame> {
    (0..count)
        .map(|i| AudioFrame {
            sequence: i as u64,
            timestamp_us: i as u64 * 10_000,
            playout_ts: 0,
            codec: AudioCodec::Flac,
            channels: 2,
            sample_rate: 48000,
            payload: Bytes::from(vec![0xABu8; 128]),
        })
        .collect()
}

#[tokio::test]
async fn loopback_stream_delivers_all_frames() {
    let frames = test_frames(50);
    let source = VecAudioSource::new(frames.clone());

    let server_addr: SocketAddr = "127.0.0.1:0".parse().expect("valid addr");
    let mut server = StreamServer::bind(server_addr).expect("server bind should succeed");
    let bound_addr = server.local_addr().expect("should have local addr");

    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

    let server_handle = tokio::spawn(async move {
        server.run(source, cancel_rx).await;
    });

    // Give server a moment to start accepting
    tokio::time::sleep(Duration::from_millis(50)).await;

    let client_addr: SocketAddr = "127.0.0.1:0".parse().expect("valid addr");
    let client = StreamClient::new(client_addr).expect("client should bind");
    let mut session = client
        .connect(bound_addr, "localhost")
        .await
        .expect("connect should succeed");

    let accept = session
        .negotiate(
            vec![AudioCodec::Flac, AudioCodec::Pcm],
            vec![44100, 48000],
            vec![2],
        )
        .await
        .expect("negotiate should succeed");

    assert_eq!(accept.codec, AudioCodec::Flac);
    assert_eq!(accept.sample_rate, 48000);
    assert_eq!(accept.channels, 2);

    // Run client session with a timeout
    let client_cancel_rx = cancel_tx.subscribe();
    let received = tokio::time::timeout(Duration::from_secs(5), session.run(client_cancel_rx))
        .await
        .expect("should not timeout")
        .expect("client run should succeed");

    // All frames should be received
    assert_eq!(
        received.len(),
        frames.len(),
        "should receive all {} frames, got {}",
        frames.len(),
        received.len()
    );

    // Verify frame contents match
    for (i, frame) in received.iter().enumerate() {
        assert_eq!(frame.sequence, i as u64);
        assert_eq!(frame.codec, AudioCodec::Flac);
        assert_eq!(frame.channels, 2);
        assert_eq!(frame.sample_rate, 48000);
    }

    let _ = cancel_tx.send(true);
    let _ = tokio::time::timeout(Duration::from_secs(2), server_handle).await;
}

#[tokio::test]
async fn session_negotiation_selects_best_codec() {
    let source = VecAudioSource::new(vec![]);

    let server_addr: SocketAddr = "127.0.0.1:0".parse().expect("valid addr");
    let mut server = StreamServer::bind(server_addr).expect("server bind should succeed");
    let bound_addr = server.local_addr().expect("should have local addr");

    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

    let server_handle = tokio::spawn(async move {
        server.run(source, cancel_rx).await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client_addr: SocketAddr = "127.0.0.1:0".parse().expect("valid addr");
    let client = StreamClient::new(client_addr).expect("client should bind");
    let mut session = client
        .connect(bound_addr, "localhost")
        .await
        .expect("connect should succeed");

    // Only offer PCM
    let accept = session
        .negotiate(vec![AudioCodec::Pcm], vec![44100], vec![2, 6])
        .await
        .expect("negotiate should succeed");

    assert_eq!(accept.codec, AudioCodec::Pcm);
    assert_eq!(accept.sample_rate, 44100);
    assert_eq!(accept.channels, 2);

    let _ = cancel_tx.send(true);
    let _ = tokio::time::timeout(Duration::from_secs(2), server_handle).await;
}

#[tokio::test]
async fn clock_sync_converges_on_loopback() {
    use syndesis::clock::ClockEstimator;

    let mut estimator = ClockEstimator::new();

    // Simulate loopback with near-zero network delay
    for i in 0..20u64 {
        let base = i * 50_000;
        let originate = base;
        let receive = base + 100;
        let transmit = base + 150;
        let destination = base + 250;
        estimator.record_exchange(originate, receive, transmit, destination);
    }

    let offset = estimator.offset_us();
    assert!(
        offset.unsigned_abs() < 1000,
        "loopback offset should be <1ms, got {offset}us"
    );
    assert!(estimator.is_stable(), "should be stable after 20 samples");
}

#[tokio::test]
async fn clock_sync_loopback_within_5ms() {
    use syndesis::clock::ClockEstimator;

    let mut estimator = ClockEstimator::new();

    // Simulate loopback: 50 samples with small jitter (< 100us RTT)
    for i in 0..50u64 {
        let base = i * 30_000;
        let jitter = (i % 7) * 5;
        let originate = base;
        let receive = base + 80 + jitter;
        let transmit = base + 90 + jitter;
        let destination = base + 170 + jitter * 2;
        estimator.record_exchange(originate, receive, transmit, destination);
    }

    let offset = estimator.offset_us();
    assert!(
        offset.unsigned_abs() < 5000,
        "loopback offset should be <5ms, got {offset}us"
    );
    assert!(estimator.is_stable(), "should be stable after 50 samples");
    assert_eq!(estimator.sample_count(), 50);
}

#[tokio::test]
async fn zone_stream_fan_out_two_renderers() {
    use syndesis::protocol::AudioCodec;
    use syndesis::server::ZoneStream;

    let mut zone = ZoneStream::new();
    let mut rx1 = zone.add_renderer("renderer-1");
    let mut rx2 = zone.add_renderer("renderer-2");

    // Feed clock sync data so coordinator has estimates
    for i in 0..10u64 {
        let base = i * 100_000;
        zone.record_clock_exchange("renderer-1", base, base + 200, base + 300, base + 500);
        zone.record_clock_exchange("renderer-2", base, base + 300, base + 400, base + 700);
    }

    // Fan out a frame
    let frame = AudioFrame {
        sequence: 42,
        timestamp_us: 5_000_000,
        playout_ts: 0,
        codec: AudioCodec::Pcm,
        channels: 2,
        sample_rate: 48000,
        payload: Bytes::from_static(b"sync-test-data"),
    };
    zone.fan_out_frame(frame).await;

    // Both renderers should receive the encoded frame
    let data1 = rx1.try_recv();
    let data2 = rx2.try_recv();
    assert!(data1.is_ok(), "renderer-1 should receive frame");
    assert!(data2.is_ok(), "renderer-2 should receive frame");

    // Decode and verify playout_ts was set
    use syndesis::protocol::codec::decode_frame;
    use syndesis::protocol::frame::Frame;

    let mut buf1 = data1.unwrap();
    let decoded = decode_frame(&mut buf1).expect("should decode");
    if let Frame::Audio(af) = decoded {
        assert!(af.playout_ts > 0, "playout_ts should be set by coordinator");
        assert_eq!(af.sequence, 42);
    } else {
        panic!("expected audio frame");
    }
}

#[tokio::test]
async fn zone_stream_sync_point_mid_stream() {
    use syndesis::protocol::AudioCodec;
    use syndesis::server::ZoneStream;

    let mut zone = ZoneStream::new();
    let _rx1 = zone.add_renderer("r1");

    // Simulate some streaming
    for i in 0..5 {
        let frame = AudioFrame {
            sequence: i,
            timestamp_us: i * 10_000,
            playout_ts: 0,
            codec: AudioCodec::Pcm,
            channels: 2,
            sample_rate: 48000,
            payload: Bytes::from_static(b"data"),
        };
        zone.fan_out_frame(frame).await;
    }

    // New renderer joins mid-stream
    let mut rx2 = zone.add_renderer("r2");
    let sync = zone.sync_point();
    assert_eq!(
        sync.sequence, 4,
        "sync point should reflect current position"
    );
    assert!(sync.server_time > 0);

    // Fan out next frame — both renderers should get it
    let next = AudioFrame {
        sequence: 5,
        timestamp_us: 50_000,
        playout_ts: 0,
        codec: AudioCodec::Pcm,
        channels: 2,
        sample_rate: 48000,
        payload: Bytes::from_static(b"new-data"),
    };
    zone.fan_out_frame(next).await;
    assert!(
        rx2.try_recv().is_ok(),
        "new renderer should receive frames after join"
    );
}

#[tokio::test]
async fn coordinator_playout_timestamps_within_5ms() {
    use syndesis::clock::ClockCoordinator;

    let mut coord = ClockCoordinator::with_margin(0);
    coord.add_renderer("r1");
    coord.add_renderer("r2");

    // Feed identical loopback-like exchanges to both renderers
    for i in 0..20u64 {
        let base = i * 50_000;
        coord.record_exchange("r1", base, base + 100, base + 150, base + 250);
        coord.record_exchange("r2", base, base + 120, base + 170, base + 290);
    }

    let server_ts = 10_000_000u64;
    let playout = coord.compute_playout_ts(server_ts).expect("should compute");

    // With near-zero offsets on loopback, playout should be close to server_ts
    let delta = playout.abs_diff(server_ts);
    assert!(
        delta < 5000,
        "playout should be within 5ms of server time on loopback, delta={delta}us"
    );
}
