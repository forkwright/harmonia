/// Client session: negotiation, frame reception, clock sync, status reporting.
use bytes::Bytes;
use snafu::ResultExt;
use tracing::{debug, instrument, trace};

use crate::client::buffer::JitterBuffer;
use crate::clock::ClockEstimator;
use crate::error::{self, SyndesisError};
use crate::protocol::codec::{decode_datagram, decode_frame, encode_datagram, encode_frame};
use crate::protocol::frame::{
    AudioFrame, ClockSync, ClockSyncReply, Frame, SessionAccept, SessionInit, StatusReport,
};
use crate::protocol::{AudioCodec, DeviceState, PROTOCOL_VERSION};
use crate::server::session::current_time_us;

const STATUS_REPORT_INTERVAL_MS: u64 = 1000;

pub struct ClientSession {
    conn: quinn::Connection,
    pub(crate) session_id: u64,
    pub(crate) buffer: JitterBuffer,
    clock: ClockEstimator,
    renderer_id: Vec<u8>,
    negotiated_codec: AudioCodec,
    negotiated_sample_rate: u32,
    negotiated_channels: u8,
}

impl ClientSession {
    pub(crate) fn new(conn: quinn::Connection) -> Self {
        Self {
            conn,
            session_id: 0,
            buffer: JitterBuffer::new(),
            clock: ClockEstimator::new(),
            renderer_id: b"default".to_vec(),
            negotiated_codec: AudioCodec::Flac,
            negotiated_sample_rate: 48000,
            negotiated_channels: 2,
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
        let _ = send.finish();

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
    /// Received frames are placed in the jitter buffer.
    #[instrument(skip_all, fields(session_id = self.session_id))]
    pub async fn run(
        &mut self,
        cancel: tokio::sync::watch::Receiver<bool>,
    ) -> Result<Vec<AudioFrame>, SyndesisError> {
        let mut recv_stream = self
            .conn
            .accept_uni()
            .await
            .context(error::ConnectionSnafu)?;
        let mut collected = Vec::new();

        let mut status_interval =
            tokio::time::interval(std::time::Duration::from_millis(STATUS_REPORT_INTERVAL_MS));
        status_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let mut read_buf = vec![0u8; 65536];

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
                            let mut data = Bytes::copy_from_slice(&read_buf[..n]);
                            while data.len() >= 4 {
                                match decode_frame(&mut data) {
                                    Ok(Frame::Audio(frame)) => {
                                        self.buffer.insert(frame.clone());
                                        collected.push(frame);
                                    }
                                    Ok(other) => {
                                        debug!(?other, "unexpected frame on audio stream");
                                    }
                                    Err(e) => {
                                        trace!(error = %e, "partial frame in read buffer");
                                        break;
                                    }
                                }
                            }
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

        Ok(collected)
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
