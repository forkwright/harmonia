# Harmonia: lexicon

*Living registry. Updated as subsystems are added or renamed.*
*For the naming methodology and construction system, see [gnomon.md](gnomon.md).*

---

## Project name

**Harmonia** (ἁρμονία): The fitting-together of disparate things into a concordant whole.

| Layer | Reading |
|-------|---------|
| L1 | Unified media platform: backend crates, audio core, desktop app |
| L2 | The integration layer that makes disparate media components work as one |
| L3 | The fitting-together of parts, not mere compatibility but active concordance, each part finding its place within a whole |
| L4 | The platform harmonizes: backend subsystems and Akouo (hearing) joined into a coherent whole. The name describes its own architecture |

---

## Components

| Name | Greek | Pronunciation | L3 Essential Nature |
|------|-------|--------------|---------------------|
| **Harmonia** | ἁρμονία | har-MOH-nee-ah | The platform as a whole: the fitting-together |
| **Akouo** | ἀκούω | ah-KOO-oh | To hear: the player. Desktop (Tauri), with Android and web planned. |

---

## Backend crates

### Infrastructure

| Crate | Greek | Pronunciation | Over | L3 Essential Nature |
|-------|-------|--------------|------|---------------------|
| **themelion** | θεμέλιον | the-MEH-lee-on | "shared types" | Foundation stone: domain primitives, IDs, and shared types used across all crates. |
| **apotheke** | ἀποθήκη | ah-po-THAY-kay | "storage" | Storehouse: SQLite storage layer, migrations, and query interface. |
| **archon** | ἄρχων | AR-khon | "server" | Ruler/assembler: Axum HTTP server and binary entry point. The process boundary. |
| **horismos** | ὁρισμός | hor-is-MOS | "config" | Definition, delimitation: the act of setting boundaries. All system configuration as the single parameterized source of truth. |
| **exousia** | ἐξουσία | ex-oo-SEE-ah | "auth" | Authority: the power that comes from legitimate standing. Identity, authentication, authorization for household users. |

### Acquisition pipeline

| Crate | Greek | Pronunciation | Over | L3 Essential Nature |
|-------|-------|--------------|------|---------------------|
| **zetesis** | ζήτησις | zay-TAY-sis | "indexer search" | The act of seeking, of inquiry: Socratic inquiry is zetesis. Queries Torznab/Newznab indexers, handles Cloudflare bypass. |
| **ergasia** | ἐργασία | er-GAH-see-ah | "download" | Working, operation, from ἔργον (the deed). Pure execution. Torrent downloads and archive extraction. |
| **syntaxis** | σύνταξις | syn-TAK-sis | "queue" | Arrangement-together, coordination: the ordering of parts into a whole. Download queue, priority, post-processing pipeline. |

### Recognition & organization

| Crate | Greek | Pronunciation | Over | L3 Essential Nature |
|-------|-------|--------------|------|---------------------|
| **epignosis** | ἐπίγνωσις | ep-ee-GNOH-sis | "metadata" | Precise knowledge, recognition: knowing in full, not mere acquaintance. Metadata enrichment from MusicBrainz, TMDB, TVDB, Audnexus. |
| **kathodos** | κάθοδος | KA-tho-dos | "import" | Descent into proper place: bringing things into their ordered place. File import, renaming, directory structure. |
| **komide** | κομιδή | ko-mee-DAY | "feeds" | Care, tending: the faithful attendance to what arrives. RSS/Atom feed aggregation for podcasts and news. |

### Quality

| Crate | Greek | Pronunciation | Over | L3 Essential Nature |
|-------|-------|--------------|------|---------------------|
| **kritike** | κριτική | kree-tee-KAY | "curation" | The critical faculty: the art of separating the excellent from the merely adequate. Library quality, integrity verification, cleanup rules. |

### Serving & integration

| Crate | Greek | Pronunciation | Over | L3 Essential Nature |
|-------|-------|--------------|------|---------------------|
| **paroche** | παροχή | pah-ro-KAY | "streaming" | Provision, supply: the act of making available. HTTP streaming, OPDS feeds, transcoding. |
| **syndesmos** | σύνδεσμος | syn-DES-mos | "external APIs" | The ligament, the bond: that which connects distinct bodies. Single integration boundary for Plex, Last.fm, Tidal. |

### Household

| Crate | Greek | Pronunciation | Over | L3 Essential Nature |
|-------|-------|--------------|------|---------------------|
| **aitesis** | αἴτησις | eye-TAY-sis | "requests" | The act of asking: a formal request, not casual speech. Household media request workflow: submission, approval, tracking. |

---

## Audio engine

| Crate | Greek | Pronunciation | Over | L3 Essential Nature |
|-------|-------|--------------|------|---------------------|
| **akouo-core** | ἀκούω | ah-KOO-oh | "audio engine" | The core listening apparatus: decode, DSP, and native audio output. Shared via FFI with the desktop app. Built independently (excluded from workspace). |

---

## Key topological relationships

- **Backend → Akouo:** Backend manages media, Akouo plays it. Neither suffices alone. Harmonia is the claim that both are necessary.
- **Zetesis → Ergasia → Syntaxis → Kathodos:** The acquisition pipeline: seek → work → coordinate → descend into place.
- **Kritike → Zetesis:** Quality assessment re-enters the acquisition pipeline for upgrades.
- **Horismos ← (all):** Configuration is the ground on which all crates stand.

See [architecture/subsystems.md](architecture/subsystems.md) for the full dependency graph with mermaid diagrams.

---

## Rejected names

| Name | Meaning | Why Rejected |
|------|---------|-------------|
| **Pheme** (Φήμη) | Rumor, report | System is about concordance, not hearsay. |
| **Chrematistike** (χρηματιστική) | Money-making | Was considered for the download/acquisition pipeline; too narrow. |
