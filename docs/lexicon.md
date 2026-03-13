# Harmonia — Lexicon

*Living registry. Updated as subsystems are added or renamed.*
*For the naming methodology and construction system, see [gnomon.md](gnomon.md).*

---

## Project Name

**Harmonia** (ἁρμονία) — The fitting-together of disparate things into a concordant whole.

| Layer | Reading |
|-------|---------|
| L1 | Unified media platform — backend, player, audio core |
| L2 | The integration layer that makes disparate media components work as one |
| L3 | The fitting-together of parts — not mere compatibility but active concordance, each part finding its place within a whole |
| L4 | The platform harmonizes: Mouseion (collection) and Akroasis (listening) joined into a coherent whole. The name describes its own architecture |

---

## Components

| Name | Greek | Pronunciation | L3 Essential Nature |
|------|-------|--------------|---------------------|
| **Harmonia** | ἁρμονία | har-MOH-nee-ah | The platform as a whole — the fitting-together |
| **Mouseion** | Μουσεῖον | moo-SAY-on | The place where the Muses dwell — custodianship of collected arts. The Rust backend. |
| **Akroasis** | ἀκρόασις | ah-kroh-AH-sis | Attentive reception — the player. Android (Kotlin/Compose), web (React), desktop (Tauri). |

---

## Backend Subsystems (Mouseion)

### Acquisition Pipeline

| Subsystem | Greek | Pronunciation | Over | L3 Essential Nature |
|-----------|-------|--------------|------|---------------------|
| **Episkope** | ἐπισκοπή | ep-ee-sko-PAY | "monitoring" | Oversight, superintendence — the episcopal function of watching over a domain. Monitors wanted media, triggers acquisition when gaps are found. |
| **Zetesis** | ζήτησις | zay-TAY-sis | "indexer search" | The act of seeking, of inquiry — Socratic inquiry is zetesis. Queries Torznab/Newznab indexers, handles Cloudflare bypass. |
| **Ergasia** | ἐργασία | er-GAH-see-ah | "download" | Working, operation — from ἔργον (the deed). Pure execution. Torrent downloads and archive extraction. |
| **Syntaxis** | σύνταξις | syn-TAK-sis | "queue" | Arrangement-together, coordination — the ordering of parts into a whole. Download queue, priority, post-processing pipeline. |

### Recognition & Organization

| Subsystem | Greek | Pronunciation | Over | L3 Essential Nature |
|-----------|-------|--------------|------|---------------------|
| **Epignosis** | ἐπίγνωσις | ep-ee-GNOH-sis | "metadata" | Precise knowledge, recognition — knowing in full, not mere acquaintance. Metadata enrichment from MusicBrainz, TMDB, TVDB, Audnexus. |
| **Taxis** | τάξις | TAK-sis | "import" | Arrangement, order — bringing things into their proper place. File import, renaming, directory structure. |

### Quality & Supplements

| Subsystem | Greek | Pronunciation | Over | L3 Essential Nature |
|-----------|-------|--------------|------|---------------------|
| **Kritike** | κριτική | kree-tee-KAY | "curation" | The critical faculty — the art of separating the excellent from the merely adequate. Library quality, integrity verification, cleanup rules. |
| **Prostheke** | προσθήκη | pros-THAY-kay | "subtitles" | The supplement, the addition — that which is added to make something more. Subtitle acquisition, sync, and storage. |

### Serving

| Subsystem | Greek | Pronunciation | Over | L3 Essential Nature |
|-----------|-------|--------------|------|---------------------|
| **Paroche** | παροχή | pah-ro-KAY | "streaming" | Provision, supply — the act of making available. HTTP streaming, OPDS feeds, transcoding. |
| **Syndesis** | σύνδεσις | syn-DEH-sis | "QUIC transport" | Binding together — server-to-renderer audio transport, multi-room clock sync, renderer discovery. |

### Household

| Subsystem | Greek | Pronunciation | Over | L3 Essential Nature |
|-----------|-------|--------------|------|---------------------|
| **Aitesis** | αἴτησις | eye-TAY-sis | "requests" | The act of asking — a formal request, not casual speech. Household media request workflow: submission, approval, tracking. |

### Cross-Cutting

| Subsystem | Greek | Pronunciation | Over | L3 Essential Nature |
|-----------|-------|--------------|------|---------------------|
| **Horismos** | ὁρισμός | hor-is-MOS | "config" | Definition, delimitation — the act of setting boundaries. All system configuration as the single parameterized source of truth. |
| **Exousia** | ἐξουσία | ex-oo-SEE-ah | "auth" | Authority — the power that comes from legitimate standing. Identity, authentication, authorization for household users. |
| **Aggelia** | ἀγγελία | an-geh-LEE-ah | "events" | The message, the announcement — past-tense facts about what has occurred. Internal event bus between subsystems. |
| **Syndesmos** | σύνδεσμος | syn-DES-mos | "external APIs" | The ligament, the bond — that which connects distinct bodies. Single integration boundary for Plex, Last.fm, Tidal. |

---

## Frontend Domains (Akroasis)

| Domain | Greek | Pronunciation | Over | L3 Essential Nature |
|--------|-------|--------------|------|---------------------|
| **Prosopon** | πρόσωπον | PROS-oh-pon | "now-playing" | The face, the mask — the persona turned toward the listener. Playback interface, controls, queue. |
| **Theoria** | θεωρία | theh-oh-REE-ah | "library" | Contemplative beholding — seeing that involves the whole mind. Library browsing and search. |
| **Heuresis** | εὕρεσις | hew-REH-sis | "discovery" | Finding, discovery — the moment of encounter with something not previously known. Recommendations, related artists, unplayed items. |
| **Boulesis** | βούλησις | boo-LAY-sis | "requests" | Rational, deliberate desire — wanting that involves will and judgment. Request submission and tracking. |
| **Diatheesis** | διάθεσις | dee-AH-theh-sis | "settings" | Disposition, arrangement — how one is arranged to respond. User preferences, playback configuration. |

---

## Key Topological Relationships

- **Mouseion ↔ Akroasis** — Collection and listening. Neither suffices alone. Harmonia is the claim that both are necessary.
- **Episkope → Zetesis → Ergasia → Syntaxis → Taxis** — The acquisition pipeline: watch → seek → work → coordinate → arrange.
- **Kritike → Episkope** — The only upward feedback edge. Quality assessment re-enters the acquisition pipeline for upgrades.
- **Aitesis ↔ Boulesis** — Backend receives what frontend wills. Together they constitute the full request arc.
- **Paroche ↔ Syndesis** — Parallel serving layers. HTTP for catalog clients, QUIC for native audio renderers.
- **Horismos ← (all)** — Configuration is the ground on which all subsystems stand.

See [architecture/subsystems.md](architecture/subsystems.md) for the full dependency graph with mermaid diagrams.

---

## Rejected Names

| Name | Meaning | Why Rejected |
|------|---------|-------------|
| **Pheme** (Φήμη) | Rumor, report | System is about concordance, not hearsay. |
| **Chrematistike** (χρηματιστική) | Money-making | Was considered for the download/acquisition pipeline; too narrow. |
