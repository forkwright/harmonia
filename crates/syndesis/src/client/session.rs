/// Client session: negotiation, frame reception, clock sync, status reporting.
use bytes::{Bytes, BytesMut};
use snafu::ResultExt;
use tracing::{debug, instrument, trace, warn};

use crate::client::buffer::JitterBuffer;
use crate::clock::ClockEstimator;
use crate::config::{ClientConfig, ClockConfig};
use crate::error::{self, SyndesisError};
use crate::protocol::codec::{
    MAX_FRAME_SIZE, decode_datagram, decode_frame, encode_datagram, encode_frame, peek_frame_len,
};
use crate::protocol::frame::{
    ClockSync, ClockSyncReply, Frame, SessionAccept, SessionInit, StatusReport,
};
use crate::protocol::{AudioCodec, DeviceState, PROTOCOL_VERSION};
use crate::server::session::current_time_us;

pub struct ClientSession {
    conn: quinn::Connection,
    pub(crate) session_id: u64,
    pub(crate) buffer: JitterBuffer,
    clock: ClockEstimator,
    renderer_id: Vec<u8>,
    negotiated_codec: AudioCodec,
    negotiated_sample_rate: u32,
    negotiated_channels: u8,
    client_config: ClientConfig,
}

impl ClientSession {
    pub(crate) fn new(conn: quinn::Connection) -> Self {
        Self::with_configs(conn, ClientConfig::default(), ClockConfig::default())
    }

    pub(crate) fn with_configs(
        conn: quinn::Connection,
        client_config: ClientConfig,
        clock_config: ClockConfig,
    ) -> Self {
        Self {
            conn,
            session_id: 0,
            buffer: JitterBuffer::with_config(&client_config),
            clock: ClockEstimator::with_config(clock_config),
            renderer_id: b"default".to_vec(),
            negotiated_codec: AudioCodec::Flac,
            negotiated_sample_rate: 48000,
            negotiated_channels: 2,
            client_config,
        }
    }

    /// Negotiate a session with the server.
    #[instrument(skip_all)]
    pub async fn negotiate(
        &mut self,
        codecs: Vec<AudioCodec>,
        sample_rates: Vec<u32>,
        channel_configs: Vec<u8>,
    ) -> Result<SessionAccept, SyndesisError> {
        let init = Frame::SessionInit(SessionInit {
            protocol_version: PROTOCOL_VERSION,
            supported_codecs: codecs,
            sample_rates,
            channel_configs,
        });

        let (mut send, mut recv) = self.conn.open_bi().await.context(error::ConnectionSnafu)?;

        let encoded = encode_frame(&init);
        send.write_all(&encoded)
            .await
            .context(error::WriteStreamSnafu)?;
        // WHY: stream finish failure is non-fatal; connection may already be reset by peer
        send.finish().ok();

        let resp_data = recv
            .read_to_end(4096)
            .await
            .context(error::ReadToEndSnafu)?;
        let mut resp_bytes = Bytes::from(resp_data);
        let frame = decode_frame(&mut resp_bytes)?;

        let Frame::SessionAccept(accept) = frame else {
            return Err(error::NegotiationSnafu {
                reason: "expected SessionAccept frame",
            }
            .build());
        };

        self.session_id = accept.session_id;
        self.negotiated_codec = accept.codec;
        self.negotiated_sample_rate = accept.sample_rate;
        self.negotiated_channels = accept.channels;

        debug!(
            session_id = self.session_id,
            ?accept.codec,
            accept.sample_rate,
            accept.channels,
            "session established"
        );

        Ok(accept)
    }

    /// Receive audio frames and handle clock sync. Runs until the stream ends or cancel fires.
    /// Received frames are placed in the jitter buffer; drain via [`Self::buffer_mut`].
    #[instrument(skip_all, fields(session_id = self.session_id))]
    pub async fn run(
        &mut self,
        cancel: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), SyndesisError> {
        let mut recv_stream = self
            .conn
            .accept_uni()
            .await
            .context(error::ConnectionSnafu)?;

        let mut status_interval = tokio::time::interval(std::time::Duration::from_millis(
            self.client_config.status_report_interval_ms,
        ));
        status_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let mut read_buf = vec![0u8; 65536];
        // WHY: frames routinely span QUIC read boundaries; bytes left over after
        // draining complete frames must persist across reads or they are lost.
        let mut stream_buf = BytesMut::new();

        loop {
            tokio::select! {
                biased;

                _ = wait_for_cancel(&cancel) => {
                    debug!("client session cancelled");
                    break;
                }

                datagram = self.conn.read_datagram() => {
                    match datagram {
                        Ok(data) => {
                            self.handle_datagram(data)?;
                        }
                        Err(e) => {
                            debug!(error = %e, "datagram read error, continuing");
                            break;
                        }
                    }
                }

                _ = status_interval.tick() => {
                    self.send_status_report()?;
                }

                result = recv_stream.read(&mut read_buf) => {
                    match result {
                        Ok(Some(n)) => {
                            stream_buf.extend_from_slice(&read_buf[..n]);
                            drain_stream_frames(&mut stream_buf, &mut self.buffer)?;
                        }
                        Ok(None) => {
                            debug!("audio stream finished");
                            break;
                        }
                        Err(e) => {
                            return Err(e).context(error::ReadStreamSnafu);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn handle_datagram(&mut self, data: Bytes) -> Result<(), SyndesisError> {
        let mut buf = data;
        let frame = decode_datagram(&mut buf)?;

        match frame {
            Frame::ClockSync(sync) => {
                self.handle_clock_sync(&sync)?;
            }
            Frame::Command(cmd) => {
                debug!(?cmd, "received command");
            }
            other => {
                debug!(?other, "unexpected datagram frame");
            }
        }
        Ok(())
    }

    fn handle_clock_sync(&mut self, sync: &ClockSync) -> Result<(), SyndesisError> {
        let now = current_time_us();
        let reply = Frame::ClockSyncReply(ClockSyncReply {
            originate_ts: sync.originate_ts,
            receive_ts: now,
            transmit_ts: current_time_us(),
            destination_ts: 0,
        });
        let data = encode_datagram(&reply);
        self.conn.send_datagram(data).map_err(|e| {
            error::DatagramSnafu {
                reason: e.to_string(),
            }
            .build()
        })?;
        trace!("sent clock sync reply");
        Ok(())
    }

    fn send_status_report(&self) -> Result<(), SyndesisError> {
        let report = Frame::StatusReport(StatusReport {
            buffer_depth_ms: self.buffer.depth_ms(),
            latency_ms: self.clock.offset_us().unsigned_abs().min(u16::MAX as u64) as u16,
            device_state: DeviceState::Active,
            renderer_id: self.renderer_id.clone(),
        });
        let data = encode_datagram(&report);
        self.conn.send_datagram(data).map_err(|e| {
            error::DatagramSnafu {
                reason: e.to_string(),
            }
            .build()
        })?;
        trace!(buffer_ms = self.buffer.depth_ms(), "sent status report");
        Ok(())
    }

    /// The jitter buffer holding received audio frames.
    #[must_use]
    pub fn buffer(&self) -> &JitterBuffer {
        &self.buffer
    }

    /// Mutable jitter-buffer access for playout draining.
    pub fn buffer_mut(&mut self) -> &mut JitterBuffer {
        &mut self.buffer
    }

    /// The negotiated codec for this session.
    #[must_use]
    pub fn codec(&self) -> AudioCodec {
        self.negotiated_codec
    }

    /// The negotiated sample rate for this session.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.negotiated_sample_rate
    }

    /// The negotiated channel count for this session.
    #[must_use]
    pub fn channels(&self) -> u8 {
        self.negotiated_channels
    }
}

// WHY: free function (not a method) so reassembly is testable without a live
// quinn::Connection.
/// Drain every complete length-prefixed frame FROM `stream_buf` into `buffer`,
/// leaving any trailing partial frame in place for the next read.
fn drain_stream_frames(
    stream_buf: &mut BytesMut,
    buffer: &mut JitterBuffer,
) -> Result<(), SyndesisError> {
    while let Some(wire_len) = peek_frame_len(stream_buf) {
        if wire_len > MAX_FRAME_SIZE {
            warn!(wire_len, "declared frame length exceeds protocol maximum");
            return Err(error::ProtocolSnafu {
                reason: "declared frame length exceeds protocol maximum",
            }
            .build());
        }
        if stream_buf.len() < wire_len {
            // Partial frame: keep the bytes buffered until the next read.
            break;
        }
        let mut frame_bytes = stream_buf.split_to(wire_len).freeze();
        match decode_frame(&mut frame_bytes) {
            Ok(Frame::Audio(frame)) => {
                buffer.insert(frame);
            }
            Ok(other) => {
                debug!(?other, "unexpected frame on audio stream");
            }
            Err(e) => {
                // WHY: the frame was complete, so this is corruption, not a
                // short read — surface it instead of silently dropping bytes.
                warn!(error = %e, "corrupt frame on audio stream");
                return Err(e);
            }
        }
    }
    Ok(())
}

async fn wait_for_cancel(cancel: &tokio::sync::watch::Receiver<bool>) {
    let mut cancel = cancel.clone();
    loop {
        if *cancel.borrow() {
            return;
        }
        if cancel.changed().await.is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::codec::encode_frame;
    use crate::protocol::frame::AudioFrame;

    fn audio_frame(seq: u64, payload_len: usize) -> Frame {
        Frame::Audio(AudioFrame {
            sequence: seq,
            timestamp_us: seq * 10_000,
            playout_ts: 0,
            codec: AudioCodec::Pcm,
            channels: 2,
            sample_rate: 48000,
            payload: Bytes::from(vec![0x5Au8; payload_len]),
        })
    }

    #[test]
    fn frame_split_across_two_reads_is_reassembled() {
        // Frame larger than the 65536-byte read chunk, split at that boundary.
        let encoded = encode_frame(&audio_frame(7, 100_000));
        assert!(encoded.len() > 65536);
        let (first, second) = encoded.split_at(65536);

        let mut stream_buf = BytesMut::new();
        let mut buffer = JitterBuffer::new();

        stream_buf.extend_from_slice(first);
        drain_stream_frames(&mut stream_buf, &mut buffer).expect("partial frame is not an error");
        assert!(buffer.is_empty(), "no complete frame yet");
        assert_eq!(stream_buf.len(), 65536, "partial bytes must be retained");

        stream_buf.extend_from_slice(second);
        drain_stream_frames(&mut stream_buf, &mut buffer).expect("complete frame decodes");
        assert_eq!(buffer.len(), 1, "exactly one frame reassembled");
        assert!(stream_buf.is_empty(), "no leftover bytes");
        assert_eq!(buffer.gap_count(), 0);
    }

    #[test]
    fn read_split_inside_length_prefix_is_reassembled() {
        let encoded = encode_frame(&audio_frame(1, 64));
        let (first, second) = encoded.split_at(2);

        let mut stream_buf = BytesMut::new();
        let mut buffer = JitterBuffer::new();

        stream_buf.extend_from_slice(first);
        drain_stream_frames(&mut stream_buf, &mut buffer).expect("short prefix is not an error");
        assert!(buffer.is_empty());
        assert_eq!(stream_buf.len(), 2, "prefix bytes must be retained");

        stream_buf.extend_from_slice(second);
        drain_stream_frames(&mut stream_buf, &mut buffer).expect("complete frame decodes");
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn multiple_frames_plus_partial_drain_in_one_pass() {
        let mut stream_buf = BytesMut::new();
        stream_buf.extend_from_slice(&encode_frame(&audio_frame(0, 32)));
        stream_buf.extend_from_slice(&encode_frame(&audio_frame(1, 32)));
        let third = encode_frame(&audio_frame(2, 32));
        stream_buf.extend_from_slice(&third[..third.len() - 5]);

        let mut buffer = JitterBuffer::new();
        drain_stream_frames(&mut stream_buf, &mut buffer).expect("complete frames decode");
        assert_eq!(buffer.len(), 2, "two complete frames drained");
        assert_eq!(
            stream_buf.len(),
            third.len() - 5,
            "partial third frame retained"
        );

        stream_buf.extend_from_slice(&third[third.len() - 5..]);
        drain_stream_frames(&mut stream_buf, &mut buffer).expect("completed frame decodes");
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.gap_count(), 0);
    }

    #[test]
    fn corrupt_complete_frame_returns_error() {
        let mut encoded = BytesMut::from(encode_frame(&audio_frame(0, 32)).as_ref());
        // Corrupt the frame-type byte (index 4, right after the length prefix).
        encoded[4] = 0xFF;

        let mut buffer = JitterBuffer::new();
        let result = drain_stream_frames(&mut encoded, &mut buffer);
        assert!(result.is_err(), "corrupt complete frame must error");
        assert!(buffer.is_empty());
    }

    #[test]
    fn oversized_declared_length_returns_error() {
        let mut stream_buf = BytesMut::new();
        stream_buf.extend_from_slice(&u32::MAX.to_be_bytes());
        stream_buf.extend_from_slice(&[0u8; 16]);

        let mut buffer = JitterBuffer::new();
        let result = drain_stream_frames(&mut stream_buf, &mut buffer);
        assert!(
            result.is_err(),
            "absurd declared length must not accumulate"
        );
    }
}
