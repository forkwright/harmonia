# QUIC streaming protocol (syndesis)

> Phase 4 deliverable. See [research/R4-quic-streaming-protocol.md](../../research/R4-quic-streaming-protocol.md) for design.

This document will specify the syndesis QUIC-based streaming protocol for:
- Server → renderer audio streaming (multi-room, Pi endpoints)
- Server → native client audio streaming (desktop, Android)
- Bidirectional control and clock synchronization

Transport: Quinn (QUIC v1). Reliable stream for FLAC audio frames.
DATAGRAMs for clock sync pulses.
