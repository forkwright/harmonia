# Spec 05: competitive analysis

**Status:** Complete (research done, findings integrated into specs)
**Priority:** none
**Issues:** none

## Goal

Map the competitive landscape of unified self-hosted media managers to validate Mouseion's positioning, identify feature gaps worth closing, and surface architectural patterns worth adopting.

## Outcome

Research completed Feb 2026. 7 competitors profiled (Cinephage, MediaManager, Nefarious, Reiverr, Huntarr, Yamtrack, Stump). Findings integrated into active specs:

| Finding | Integrated Into |
|---------|----------------|
| OIDC/OAuth authentication | Spec 06 Phase 3 |
| Import from Trakt/MAL/AniList | Spec 07 Phases 1-2 (done) |
| OPDS 1.2 for books/comics | Spec 01 Phase 5 |
| Calendar + iCal | Spec 03 (future phase) |
| Rate limiting per indexer | Spec 08 Phase 2 (done) |
| .strm file generation | Spec 08 Phase 4 |
| Delay profiles | Spec 03 Phase 3 |
| Smart Lists | Spec 03 Phase 2 |
| Media server auto-tracking | Spec 01 Phase 4 |
| Download client abstraction | Spec 08 Phase 1 (done) |

Full competitive profiles, feature matrix, and strategic analysis preserved in git history for reference.

## Notes

- Re-evaluate landscape in 3 months (May 2026). Cinephage and Huntarr growing fastest.
- Mouseion's moat: 10+ native media types, full lifecycle (discovery→acquisition→organization→tracking→serving), C#/.NET performance.
