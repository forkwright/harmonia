/// Per-renderer server session: negotiation, audio streaming, clock sync, flow control.
use bytes::Bytes;
use snafu::ResultExt;
use tracing::{debug, instrument, trace};

use crate::clock::{ClockEstimator, SyncScheduler};
use crate::config::{ClockConfig, ServerConfig};
use crate::error::{self, SyndesisError};
use crate::protocol::codec::{decode_datagram, decode_frame, encode_datagram, encode_frame};
use crate::protocol::frame::{
    ClockSync, ClockSyncReply, Frame, SessionAccept, SessionInit, StatusReport,
};
use crate::protocol::{AudioCodec, PROTOCOL_VERSION};
use crate::server::source::AudioSource;

pub struct StreamSession {
    conn: quinn::Connection,
    session_id: u64,
    clock: ClockEstimator,
    scheduler: SyncScheduler,
    is_paused: bool,
    server_config: ServerConfig,
}

impl StreamSession {
    pub(crate) fn with_configs(
        conn: quinn::Connection,
        server_config: ServerConfig,
        clock_config: ClockConfig,
    ) -> Self {
        Self {
            conn,
            session_id: 0,
            clock: ClockEstimator::with_config(clock_config.clone()),
            scheduler: SyncScheduler::with_config(clock_config),
            is_paused: false,
            server_config,
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
                    sync_interval = delayed_interval(new_interval);
                }

                frame = source.next_frame(), if !self.is_paused && !source_exhausted => {
                    match frame {
                        Some(audio_frame) => {
                            let encoded = encode_frame(&Frame::Audio(audio_frame));
                            send_stream.write_all(&encoded).await // kanon:ignore RUST/select-cancel-safety -- biased select checks cancel first; stream teardown on drop
                                .context(error::WriteStreamSnafu)?;
                        }
                        None => {
                            debug!("audio source exhausted");
                            // WHY: stream finish failure is non-fatal; connection may already be reset by peer
                            send_stream.finish().ok();
                            source_exhausted = true;
                        }
                    }
                }
            }
        }

        if !source_exhausted {
            // WHY: stream finish failure is non-fatal; connection may already be reset by peer
            send_stream.finish().ok();
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
        // WHY: stream finish failure is non-fatal; connection may already be reset by peer
        send.finish().ok();

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
        if report.buffer_depth_ms > self.server_config.buffer_high_watermark_ms && !self.is_paused {
            debug!(
                buffer_ms = report.buffer_depth_ms,
                "pausing stream: buffer above high watermark"
            );
            self.is_paused = true;
        } else if report.buffer_depth_ms < self.server_config.buffer_low_watermark_ms
            && self.is_paused
        {
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

    // WHY: Server preference ORDER: FLAC first (lossless), then PCM as fallback.
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
        .unwrap_or_default() // WHY: SystemTime cannot be before UNIX_EPOCH on any supported platform
        .as_micros() as u64
}

// WHY: tokio::time::interval fires its first tick immediately; a rebuilt
// interval must instead wait a full period or every scheduler update would
// fire an instant extra probe, defeating the computed backoff.
fn delayed_interval(period: std::time::Duration) -> tokio::time::Interval {
    let mut interval = tokio::time::interval_at(tokio::time::Instant::now() + period, period);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    interval
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
    use std::time::Duration;

    use super::*;

    fn init(codecs: Vec<AudioCodec>, rates: Vec<u32>, channels: Vec<u8>) -> SessionInit {
        SessionInit {
            protocol_version: PROTOCOL_VERSION,
            supported_codecs: codecs,
            sample_rates: rates,
            channel_configs: channels,
        }
    }

    #[test]
    fn negotiate_rejects_version_mismatch() {
        let mut bad = init(vec![AudioCodec::Flac], vec![48000], vec![2]);
        bad.protocol_version = PROTOCOL_VERSION + 1;
        let result = negotiate_params(&bad);
        assert!(matches!(result, Err(SyndesisError::Negotiation { .. })));
    }

    #[test]
    fn negotiate_prefers_flac_over_pcm() {
        let both = init(
            vec![AudioCodec::Pcm, AudioCodec::Flac],
            vec![48000],
            vec![2],
        );
        let (codec, rate, channels) = negotiate_params(&both).expect("negotiation succeeds");
        assert_eq!(codec, AudioCodec::Flac);
        assert_eq!(rate, 48000);
        assert_eq!(channels, 2);
    }

    #[test]
    fn negotiate_falls_back_to_pcm() {
        let pcm_only = init(vec![AudioCodec::Pcm], vec![44100], vec![2]);
        let (codec, rate, _) = negotiate_params(&pcm_only).expect("negotiation succeeds");
        assert_eq!(codec, AudioCodec::Pcm);
        assert_eq!(rate, 44100);
    }

    #[test]
    fn negotiate_rejects_no_common_codec() {
        let none = init(vec![], vec![48000], vec![2]);
        assert!(matches!(
            negotiate_params(&none),
            Err(SyndesisError::Negotiation { .. })
        ));
    }

    #[test]
    fn negotiate_rejects_unsupported_sample_rate() {
        let odd_rate = init(vec![AudioCodec::Flac], vec![22050], vec![2]);
        assert!(matches!(
            negotiate_params(&odd_rate),
            Err(SyndesisError::Negotiation { .. })
        ));
    }

    #[test]
    fn negotiate_channel_policy_prefers_stereo_then_first() {
        let multi = init(vec![AudioCodec::Flac], vec![48000], vec![6, 2]);
        let (_, _, channels) = negotiate_params(&multi).expect("negotiation succeeds");
        assert_eq!(channels, 2, "stereo preferred when offered");

        let surround_only = init(vec![AudioCodec::Flac], vec![48000], vec![6]);
        let (_, _, channels) = negotiate_params(&surround_only).expect("negotiation succeeds");
        assert_eq!(channels, 6, "first offered config when no stereo");

        let empty = init(vec![AudioCodec::Flac], vec![48000], vec![]);
        assert!(matches!(
            negotiate_params(&empty),
            Err(SyndesisError::Negotiation { .. })
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn delayed_interval_first_tick_waits_full_period() {
        let period = Duration::from_secs(5);
        let start = tokio::time::Instant::now();

        let mut interval = delayed_interval(period);
        interval.tick().await;

        let elapsed = tokio::time::Instant::now() - start;
        assert!(
            elapsed >= period,
            "first tick after {elapsed:?}, expected >= {period:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn delayed_interval_ticks_at_full_period_thereafter() {
        let period = Duration::from_secs(5);
        let mut interval = delayed_interval(period);

        interval.tick().await;
        let after_first = tokio::time::Instant::now();
        interval.tick().await;

        let gap = tokio::time::Instant::now() - after_first;
        assert!(
            gap >= period,
            "second tick gap {gap:?}, expected >= {period:?}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn plain_interval_first_tick_is_immediate() {
        // WHY: contrast case documenting the defect delayed_interval fixes.
        let period = Duration::from_secs(5);
        let start = tokio::time::Instant::now();

        let mut interval = tokio::time::interval(period);
        interval.tick().await;

        let elapsed = tokio::time::Instant::now() - start;
        assert!(
            elapsed < period,
            "tokio::time::interval fires immediately; got {elapsed:?}"
        );
    }
}
