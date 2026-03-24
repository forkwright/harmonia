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
