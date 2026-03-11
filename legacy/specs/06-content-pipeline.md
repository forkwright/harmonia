# Spec 06: Content Acquisition Pipeline

**Status:** Draft
**Priority:** Low
**Issues tracked:** #83, #85, #87, #82 (closing all — spec is source of truth)

## Goal

Automated content acquisition and library management. Download client integration, format preservation, library health, and the Akroasis Protocol spec for third-party clients.

## Phases

### Phase 1: Data model fixes — **PARTIALLY UNBLOCKED**
- [ ] Format preservation metadata in library DB
- [ ] Track → Album association improvements

### Phase 2: Acquisition — **PARTIALLY UNBLOCKED** (Mouseion has download clients: Transmission, SABnzbd, NZBGet, Deluge)
- [ ] Lidarr integration for music acquisition
- [ ] Readarr integration for audiobook/ebook acquisition
- [ ] Import pipeline (download → tag → organize → ingest)

### Phase 3: Library health — **UNBLOCKED** (LibraryStatisticsController exists)
- [ ] Library health monitoring and reporting
- [ ] Duplicate detection
- [ ] Missing metadata identification
- [ ] Format migration tools

### Phase 4: Protocol
- [ ] Publish Akroasis Protocol as open specification
- [ ] API documentation for third-party clients
- [ ] Reference client implementation notes

## Dependencies

- Mouseion now has download clients (Spec 08), indexer rate limiting, and library statistics
- Protocol spec is documentation-only — can start independently

## Notes

- Mouseion Spec 08 shipped download clients and rate limiting — acquisition is partially unblocked.
- Library health monitoring can use LibraryStatisticsController.
- Protocol spec is a documentation task — not blocked on any code.
