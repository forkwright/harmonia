# Spec 06: Content Acquisition Pipeline

**Status:** Draft
**Priority:** Low
**Issues:** #83, #85, #87, #93

## Goal

Automated content acquisition and library management. Lidarr/Readarr integration for music and audiobooks, format preservation during migration, and the Akroasis Protocol spec for third-party clients. This is the "self-hosted power user" layer.

## Phases

### Phase 1: Data model fixes
- [ ] Add albumId to Track model (#93) — blocked on Mouseion
- [ ] Format preservation metadata in library DB (#85)

### Phase 2: Acquisition
- [ ] Lidarr integration for music acquisition (#83)
- [ ] Readarr integration for audiobook/ebook acquisition (#83)
- [ ] Import pipeline (download → tag → organize → ingest)

### Phase 3: Library health
- [ ] Library health monitoring and reporting (#82)
- [ ] Duplicate detection
- [ ] Missing metadata identification
- [ ] Format migration tools (e.g., lossy → lossless replacement)

### Phase 4: Protocol
- [ ] Publish Akroasis Protocol as open specification (#87)
- [ ] API documentation for third-party clients
- [ ] Reference client implementation notes

## Dependencies

- All phases depend on Mouseion backend work
- Lidarr/Readarr integration is Mouseion-side, not client-side

## Notes

- This entire spec is blocked on Mouseion. Low priority until backend capacity opens up.
- The protocol spec (#87) is documentation-only and could be started independently.
- albumId fix (#93) is a duplicate of #54 — close one.
