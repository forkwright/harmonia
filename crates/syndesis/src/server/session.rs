/// Per-renderer server session: negotiation, audio streaming, clock sync, flow control.
use bytes::Bytes;
use snafu::ResultExt;
use tracing::{debug, instrument, trace};

use crate::clock::{ClockEstimator, SyncScheduler};
use crate::error::{self, SyndesisError};
use crate::protocol::codec::{decode_datagram, decode_frame, encode_datagram, encode_frame};
use crate::protocol::frame::{
    ClockSync, ClockSyncReply, Frame, SessionAccept, SessionInit, StatusReport,
};
use crate::protocol::{AudioCodec, PROTOCOL_VERSION};
use crate::server::source::AudioSource;

const BUFFER_HIGH_WATERMARK_MS: u16 = 200;
const BUFFER_LOW_WATERMARK_MS: u16 = 80;

pub struct StreamSession {
    conn: quinn::Connection,
    session_id: u64,
    clock: ClockEstimator,
    scheduler: SyncScheduler,
    is_paused: bool,
}

impl StreamSession {
    pub(crate) fn new(conn: quinn::Connection) -> Self {
        Self {
            conn,
            session_id: 0,
            clock: ClockEstimator::new(),
            scheduler: SyncScheduler::new(),
            is_paused: false,
        }
    }

    /// Run the session: negotiate, then stream audio while handling clock sync and status.
    #[instrument(skip_all, fields(session_id))]
    pub async fn run<S: AudioSource>(
        &mut self,
        mut source: S,
        cancel: tokio::sync::watch::Receiver<bool>,
    ) -> Result<(), SyndesisError> {
        self.negotiate().await?;
        tracing::Span::current().record("session_id", self.session_id);

        let mut send_stream = self.conn.open_uni().await.context(error::ConnectionSnafu)?;

        let mut sync_interval = tokio::time::interval(self.scheduler.interval());
        sync_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let mut source_exhausted = false;

        loop {
            tokio::select! {
                biased;

                _ = wait_for_cancel(&cancel) => {
                    debug!("session cancelled");
                    break;
                }

                datagram = self.conn.read_datagram() => {
                    match datagram {
                        Ok(data) => {
                            self.handle_datagram(data)?;
                        }
                        Err(e) => {
                            return Err(e).context(error::ConnectionSnafu);
                        }
                    }
                }

                _ = sync_interval.tick() => {
                    self.send_clock_probe()?;
                    let new_interval = self.scheduler.update(&self.clock);
                    sync_interval = tokio::time::interval(new_interval);
                    sync_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                }

                frame = source.next_frame(), if !self.is_paused && !source_exhausted => {
                    match frame {
                        Some(audio_frame) => {
                            let encoded = encode_frame(&Frame::Audio(audio_frame));
                            send_stream.write_all(&encoded).await
                                .context(error::WriteStreamSnafu)?;
                        }
                        None => {
                            debug!("audio source exhausted");
                            let _ = send_stream.finish();
                            source_exhausted = true;
                        }
                    }
                }
            }
        }

        if !source_exhausted {
            let _ = send_stream.finish();
        }
        Ok(())
    }

    async fn negotiate(&mut self) -> Result<(), SyndesisError> {
        let (mut send, mut recv) = self
            .conn
            .accept_bi()
            .await
            .context(error::ConnectionSnafu)?;

        let init_data = recv
            .read_to_end(4096)
            .await
            .context(error::ReadToEndSnafu)?;
        let mut init_bytes = Bytes::from(init_data);
        let frame = decode_frame(&mut init_bytes)?;

        let Frame::SessionInit(init) = frame else {
            return Err(error::NegotiationSnafu {
                reason: "expected SessionInit frame",
            }
            .build());
        };

        let (codec, sample_rate, channels) = negotiate_params(&init)?;

        self.session_id = rand::random();

        let accept = Frame::SessionAccept(SessionAccept {
            codec,
            sample_rate,
            channels,
            session_id: self.session_id,
        });

        let encoded = encode_frame(&accept);
        send.write_all(&encoded)
            .await
            .context(error::WriteStreamSnafu)?;
        let _ = send.finish();

        debug!(
            session_id = self.session_id,
            ?codec,
            sample_rate,
            channels,
            "session negotiated"
        );

        Ok(())
    }

    fn send_clock_probe(&self) -> Result<(), SyndesisError> {
        let now = current_time_us();
        let probe = Frame::ClockSync(ClockSync {
            originate_ts: now,
            receive_ts: 0,
            transmit_ts: 0,
        });
        let data = encode_datagram(&probe);
        self.conn.send_datagram(data).map_err(|e| {
            error::DatagramSnafu {
                reason: e.to_string(),
            }
            .build()
        })?;
        trace!("sent clock probe");
        Ok(())
    }

    fn handle_datagram(&mut self, data: Bytes) -> Result<(), SyndesisError> {
        let mut buf = data;
        let frame = decode_datagram(&mut buf)?;

        match frame {
            Frame::ClockSyncReply(reply) => self.handle_clock_reply(&reply),
            Frame::StatusReport(report) => self.handle_status_report(&report),
            other => {
                debug!(?other, "unexpected datagram frame type");
            }
        }
        Ok(())
    }

    fn handle_clock_reply(&mut self, reply: &ClockSyncReply) {
        self.clock.record_exchange(
            reply.originate_ts,
            reply.receive_ts,
            reply.transmit_ts,
            reply.destination_ts,
        );
        trace!(
            offset_us = self.clock.offset_us(),
            stable = self.clock.is_stable(),
            "clock sync updated"
        );
    }

    fn handle_status_report(&mut self, report: &StatusReport) {
        if report.buffer_depth_ms > BUFFER_HIGH_WATERMARK_MS && !self.is_paused {
            debug!(
                buffer_ms = report.buffer_depth_ms,
                "pausing stream: buffer above high watermark"
            );
            self.is_paused = true;
        } else if report.buffer_depth_ms < BUFFER_LOW_WATERMARK_MS && self.is_paused {
            debug!(
                buffer_ms = report.buffer_depth_ms,
                "resuming stream: buffer below low watermark"
            );
            self.is_paused = false;
        }
        trace!(
            buffer_ms = report.buffer_depth_ms,
            latency_ms = report.latency_ms,
            ?report.device_state,
            "status report received"
        );
    }
}

fn negotiate_params(init: &SessionInit) -> Result<(AudioCodec, u32, u8), SyndesisError> {
    if init.protocol_version != PROTOCOL_VERSION {
        return Err(error::NegotiationSnafu {
            reason: "unsupported protocol version",
        }
        .build());
    }

    // WHY: Server preference order: FLAC first (lossless), then PCM as fallback.
    let codec = if init.supported_codecs.contains(&AudioCodec::Flac) {
        AudioCodec::Flac
    } else if init.supported_codecs.contains(&AudioCodec::Pcm) {
        AudioCodec::Pcm
    } else {
        return Err(error::NegotiationSnafu {
            reason: "no supported codec",
        }
        .build());
    };

    let preferred_rates = [48000, 44100, 96000, 192000];
    let sample_rate = preferred_rates
        .iter()
        .find(|r| init.sample_rates.contains(r))
        .copied()
        .ok_or_else(|| {
            error::NegotiationSnafu {
                reason: "no supported sample rate",
            }
            .build()
        })?;

    let channels = if init.channel_configs.contains(&2) {
        2
    } else {
        *init.channel_configs.first().ok_or_else(|| {
            error::NegotiationSnafu {
                reason: "no channel config",
            }
            .build()
        })?
    };

    Ok((codec, sample_rate, channels))
}

pub(crate) fn current_time_us() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_micros() as u64
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
