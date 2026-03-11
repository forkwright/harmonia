# Subsystem Registry

> Every Greek name in Harmonia with its layer test, directory, and interface boundary.
> Names follow the gnomon naming system (see [gnomon.md](../gnomon.md)).
> Shared definitions live in [GLOSSARY.md](../GLOSSARY.md).

## Overview

Harmonia contains two top-level components: Mouseion (the backend) and Akroasis (the player). Mouseion contains 15 backend subsystems covering the full media lifecycle — from monitoring wanted media through indexer access, download, metadata enrichment, organization, serving (both HTTP and native QUIC transport), and household requests, plus cross-cutting concerns for configuration, authorization, and internal event announcements, and a single integration boundary for external API services. Akroasis contains 5 front-end domains covering the full player surface — playback, library browsing, settings, request submission, and discovery.

### Backend Subsystems (Mouseion)

| Name | Pronunciation | Primary Responsibility |
|------|--------------|----------------------|
| Aggelia | an-geh-LEE-ah | Carries internal event announcements between subsystems — past-tense facts about what has occurred |
| Episkope | ep-ee-sko-PAY | Monitors the state of wanted media and triggers acquisition when gaps are found |
| Zetesis | zay-TAY-sis | Queries indexers for available media using Torznab/Newznab protocols |
| Ergasia | er-GAH-see-ah | Executes torrent downloads and archive extraction |
| Syntaxis | syn-TAK-sis | Coordinates the download queue, priority, and post-processing pipeline |
| Epignosis | ep-ee-GNOH-sis | Enriches media with metadata from external providers |
| Taxis | TAK-sis | Imports and organizes media files into the library structure |
| Kritike | kree-tee-KAY | Assesses library quality, verifies integrity, and enforces curation rules |
| Prostheke | pros-THAY-kay | Manages subtitle acquisition, synchronization, and storage |
| Paroche | pah-ro-KAY | Serves media to clients via HTTP streaming, OPDS, and transcoding |
| Aitesis | eye-TAY-sis | Manages household member media requests through the full workflow |
| Horismos | hor-is-MOS | Owns all system configuration as the single parameterized source of truth |
| Exousia | ex-oo-SEE-ah | Manages identity, authentication, and authorization for household users |
| Syndesmos | syn-DES-mos | Connects Harmonia to external API services (Plex, Last.fm, Tidal) |
| Syndesis | syn-DEH-sis | QUIC streaming protocol — server↔renderer audio transport, multi-room clock sync, renderer discovery and pairing |

### Front-End Domains (Akroasis)

| Name | Pronunciation | Primary Responsibility |
|------|--------------|----------------------|
| Prosopon | PROS-oh-pon | The playback interface — now-playing, controls, queue |
| Theoria | theh-oh-REE-ah | Library browsing and search across the full collection |
| Diatheesis | dee-AH-theh-sis | User preferences and server configuration |
| Boulesis | boo-LAY-sis | Household member request submission and tracking |
| Heuresis | hew-REH-sis | Discovery, exploration, and recommendations |

---

## Backend Subsystems

### Episkope (ἐπισκοπή)

| Property | Value |
|----------|-------|
| Pronunciation | ep-ee-sko-PAY |
| Directory | mouseion/crates/episkope/ |
| Primary responsibility | Monitors the desired state of the media library against what exists, tracks release schedules, and triggers acquisition when gaps are found |
| Interface boundary | Exposes a wanted-media registry and trigger API; receives completion signals from Syntaxis |
| Calls | Zetesis (to initiate searches), Syntaxis (to enqueue found items), Epignosis (to verify media identity) |
| Called by | Aitesis (when a user request is approved, Episkope begins monitoring) |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Watches what you want versus what you have — when something's missing or a better version exists, it fires a search |
| L2 — Structural | The governing layer above acquisition: Zetesis, Ergasia, and Syntaxis all work in service of what Episkope has determined is wanted |
| L3 — Philosophical | ἐπισκοπή is oversight, superintendence — the episcopal function of watching over a domain and knowing its state. The ancient bishop (ἐπίσκοπος) is one who watches over a flock; Episkope watches over the media library's completeness |
| L4 — Reflexive | Episkope oversees: it keeps vigil over what is wanted, notices absence, and acts. The name is itself a vigil — it describes watchfulness by being a word for watchfulness |

---

### Zetesis (ζήτησις)

| Property | Value |
|----------|-------|
| Pronunciation | zay-TAY-sis |
| Directory | mouseion/crates/zetesis/ |
| Primary responsibility | Queries indexers via Torznab/Newznab protocols, handles Cloudflare bypass for protected trackers, and returns ranked results |
| Interface boundary | Exposes a search API keyed on media identity; manages indexer credentials and protocol negotiation |
| Calls | Horismos (for indexer credentials and configuration), Exousia (to verify caller is authorized) |
| Called by | Episkope (to search for wanted media), Syntaxis (to refresh stale searches) |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Probes indexers and trackers to find download candidates — handles the Torznab/Newznab protocol negotiation and Cloudflare bypass |
| L2 — Structural | The outward-reaching function: where Episkope knows what is wanted, Zetesis goes and asks the world if it exists |
| L3 — Philosophical | ζήτησις is the act of seeking, of inquiry — the philosophical term for the movement of thought toward what is not yet known. Socratic inquiry is zetesis. This subsystem enacts inquiry in the literal sense: it poses questions to external sources and awaits answers |
| L4 — Reflexive | Zetesis seeks: it probes, asks, queries. The name is itself the act it describes — to name it is to perform it |

---

### Ergasia (ἐργασία)

| Property | Value |
|----------|-------|
| Pronunciation | er-GAH-see-ah |
| Directory | mouseion/crates/ergasia/ |
| Primary responsibility | Executes torrent downloads, manages seeding, and extracts archives after download completion |
| Interface boundary | Exposes a download-execution API; emits completion events with file paths; manages the BitTorrent session |
| Calls | Horismos (for download directories and limits), Syntaxis (to report completion) |
| Called by | Syntaxis (to initiate or cancel downloads) |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Downloads torrents, manages the BitTorrent session, extracts archives — the actual work of getting files onto disk |
| L2 — Structural | The execution layer: Syntaxis directs it, Episkope wants what it produces. Ergasia does nothing that Syntaxis has not authorized |
| L3 — Philosophical | ἐργασία is working, operation, the carrying-out of work — from ἔργον (the deed, the work). Where other subsystems plan, direct, and assess, Ergasia simply works. It is the subsystem of pure execution |
| L4 — Reflexive | Ergasia works: it is the working. The name carries no pretension beyond naming what the subsystem does at the level of essential nature — it is the operation itself |

---

### Syntaxis (σύνταξις)

| Property | Value |
|----------|-------|
| Pronunciation | syn-TAK-sis |
| Directory | mouseion/crates/syntaxis/ |
| Primary responsibility | Manages the download queue, enforces priority and bandwidth rules, coordinates post-download processing, and triggers import |
| Interface boundary | Exposes a queue management API; orchestrates the pipeline from Zetesis results through Ergasia execution to Taxis import |
| Calls | Ergasia (to execute downloads), Taxis (to trigger import after download), Episkope (to report completion) |
| Called by | Episkope (to enqueue found items), Zetesis (to update search results) |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Manages the download queue — what downloads now, what waits, what gets cancelled; coordinates the pipeline from found-to-downloaded-to-imported |
| L2 — Structural | The coordination layer: it holds Zetesis (finding), Ergasia (downloading), and Taxis (importing) in a coherent sequence. Without Syntaxis, each subsystem operates in isolation |
| L3 — Philosophical | σύνταξις is arrangement-together, coordination — the word from which English "syntax" derives. Aristotle uses it for the ordering of parts into a whole. Syntaxis is the subsystem that puts things in order, that makes the sequence make sense |
| L4 — Reflexive | Syntaxis coordinates: it arranges the subsystems into a working sequence. The name is itself an example of coordination — it names the joining of elements (syn-) into order (taxis) |

---

### Epignosis (ἐπίγνωσις)

| Property | Value |
|----------|-------|
| Pronunciation | ep-ee-GNOH-sis |
| Directory | mouseion/crates/epignosis/ |
| Primary responsibility | Enriches media items with metadata from external providers (MusicBrainz, TMDB, TVDB, Audnexus, etc.) and maintains the metadata cache |
| Interface boundary | Exposes a metadata resolution API keyed on media identity; manages provider credentials and rate limiting |
| Calls | Horismos (for provider API keys and cache TTL), Syndesmos (for Last.fm artist data) |
| Called by | Taxis (to enrich imported media), Episkope (to verify identity of candidates) |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Fetches and stores metadata from MusicBrainz, TMDB, TVDB, and other providers — knows what an album, movie, or audiobook actually is |
| L2 — Structural | The recognition layer: before Taxis can organize something correctly, Epignosis must have identified it precisely. Epignosis is what makes the library know what it contains |
| L3 — Philosophical | ἐπίγνωσις is precise knowledge, recognition, discernment — a word used in Greek philosophy to distinguish full recognition from mere acquaintance. Where γνῶσις is knowing, ἐπίγνωσις is knowing precisely, knowing in full. This subsystem doesn't just know a file exists — it knows exactly what it is |
| L4 — Reflexive | Epignosis recognizes: it takes an ambiguous media item and returns precise knowledge of its identity. The name demonstrates this — it is itself an act of precise naming, distinguishing this kind of knowing from mere recognition |

---

### Taxis (τάξις)

| Property | Value |
|----------|-------|
| Pronunciation | TAK-sis |
| Directory | mouseion/crates/taxis/ |
| Primary responsibility | Imports downloaded media into the library, renames files according to configured schema, and establishes the directory structure |
| Interface boundary | Exposes an import API triggered by Syntaxis; reads Epignosis metadata to determine naming and placement |
| Calls | Epignosis (for metadata to drive naming), Horismos (for library root paths and naming schema), Kritike (to register imported items for curation tracking) |
| Called by | Syntaxis (after download completion) |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Moves downloaded files into the library with correct names and directory structure — the rename-and-place operation |
| L2 — Structural | The structuring layer: after Syntaxis delivers and Epignosis identifies, Taxis arranges. It is the subsystem that gives the library its form |
| L3 — Philosophical | τάξις is arrangement, order — one of the fundamental Greek words for the ordering of parts. Plato uses taxis for the ordered arrangement of a whole; it is not mere sorting but the bringing of things into their proper place |
| L4 — Reflexive | Taxis arranges: it takes unordered downloads and places them into the ordered structure of the library. The name is itself ordered — it is a clean, single word that arranges clearly |

---

### Kritike (κριτική)

| Property | Value |
|----------|-------|
| Pronunciation | kree-tee-KAY |
| Directory | mouseion/crates/kritike/ |
| Primary responsibility | Assesses library quality, verifies file integrity, enforces cleanup rules, and triggers quality upgrades |
| Interface boundary | Exposes a curation rule API and a library health API; integrates with Episkope to trigger re-acquisition for quality upgrades |
| Calls | Episkope (to trigger upgrade acquisition), Horismos (for curation rules and thresholds) |
| Called by | Taxis (to register newly imported items), internal scheduler (for periodic library scans) |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Scans the library for corrupt files, quality downgrades, and cleanup targets — triggers re-acquisition when a better version is available, deletes what should go |
| L2 — Structural | The judgment layer: after Taxis places things and the library grows, Kritike maintains its health. Every subsystem that puts things into the library is answered by Kritike's assessment |
| L3 — Philosophical | κριτική is the critical faculty, the art of discernment and judgment — from κρίνω (to separate, to judge). Literary criticism is kritike; the faculty that separates the excellent from the merely adequate. This subsystem exercises that faculty continuously against the library |
| L4 — Reflexive | Kritike judges: it separates what belongs from what does not, what is high enough quality from what must be replaced. The name itself is discriminating — it is the precise word for the critical faculty, not a synonym or approximation |

---

### Prostheke (προσθήκη)

| Property | Value |
|----------|-------|
| Pronunciation | pros-THAY-kay |
| Directory | mouseion/crates/prostheke/ |
| Primary responsibility | Acquires subtitles, synchronizes timing to audio tracks, stores subtitle files alongside media, and manages subtitle language preferences |
| Interface boundary | Exposes a subtitle-lookup and sync API keyed on media identity; manages subtitle provider credentials |
| Calls | Epignosis (for media identity to drive subtitle lookup), Horismos (for subtitle language preferences and provider credentials) |
| Called by | Taxis (after import, to trigger initial subtitle acquisition), Episkope (on quality upgrade, to refresh subtitles) |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Finds, downloads, and syncs subtitle tracks for media — covers multiple languages and subtitle sources |
| L2 — Structural | The supplement layer: it adds to what Taxis has organized. Prostheke is always secondary to the primary media — it exists in relation to something else |
| L3 — Philosophical | προσθήκη is an addition, a supplement, an appendage — from προστίθημι (to add, to place alongside). A subtitle is precisely a prostheke: something added to primary media that did not originate with it, that supplements it for accessibility or preference |
| L4 — Reflexive | Prostheke is itself an addition to the registry — a subsystem that would not exist if subtitles were not a separately manageable concern. The name describes its own relationship to the media it serves |

---

### Paroche (παροχή)

| Property | Value |
|----------|-------|
| Pronunciation | pah-ro-KAY |
| Directory | mouseion/crates/paroche/ |
| Primary responsibility | Serves media to clients via HTTP streaming, OPDS catalog feeds, and on-demand transcoding |
| Interface boundary | Exposes HTTP streaming endpoints, OPDS feeds, and a transcoding API; reads from the library directly |
| Calls | Exousia (to authorize incoming requests), Horismos (for transcoding profiles and stream limits) |
| Called by | Akroasis clients (for streaming), external catalog readers (for OPDS) |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Streams audio, books, and comics to clients — also serves OPDS catalog feeds and handles transcoding for unsupported formats |
| L2 — Structural | The outward-facing service layer: where all other backend subsystems build and maintain the library inward, Paroche delivers it outward to clients |
| L3 — Philosophical | παροχή is provision, supply, furnishing — from παρέχω (to provide, to furnish). It names the act of making available, of providing what is needed. A paroche is not a transaction but a continuous provision |
| L4 — Reflexive | Paroche provides: it is the providing function of the system. The name is itself an act of provision — it gives a precise word for what would otherwise be called "the server" or "the streaming layer," furnishing a name that names the essential nature of furnishing |

---

### Aitesis (αἴτησις)

| Property | Value |
|----------|-------|
| Pronunciation | eye-TAY-sis |
| Directory | mouseion/crates/aitesis/ |
| Primary responsibility | Manages the household media request workflow — submission, approval, status tracking, and handoff to Episkope |
| Interface boundary | Exposes a request submission and management API; integrates with Exousia for per-user request limits |
| Calls | Episkope (to begin monitoring approved requests), Epignosis (to validate requested media identity), Exousia (for request authorization limits) |
| Called by | Akroasis Boulesis (the front-end request domain), external request sources |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | The request system — household members submit what they want, track status, and the system handles approval and acquisition handoff |
| L2 — Structural | The inbound human interface: where Episkope monitors the system's own knowledge of what is wanted, Aitesis receives wants from outside the system |
| L3 — Philosophical | αἴτησις is the act of asking, of requesting — from αἰτέω (to ask, to demand). In Aristotle it is the formal request, the claim made upon another. Aitesis names the subsystem by its essential function: it is the asking |
| L4 — Reflexive | Aitesis asks: it is the system's capacity to receive and process the asking of others. The name performs the act it describes — it is itself a word for asking |

---

### Horismos (ὁρισμός)

| Property | Value |
|----------|-------|
| Pronunciation | hor-is-MOS |
| Directory | mouseion/crates/horismos/ |
| Primary responsibility | Owns all system configuration as the single parameterized source of truth — no other subsystem hardcodes values that belong here |
| Interface boundary | Exposes a typed configuration API; manages secrets, environment overrides, and configuration validation |
| Calls | None — Horismos is a leaf dependency |
| Called by | All other subsystems (for configuration values) |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | All configuration lives here — API keys, paths, quality thresholds, limits. Other subsystems read from Horismos; they do not own their own configuration |
| L2 — Structural | The foundational layer: Horismos has no upstream dependencies. Every other subsystem depends on it. It is the ground |
| L3 — Philosophical | ὁρισμός is definition, delimitation, the bounding of a thing — from ὁρίζω (to bound, to define, to determine). In Aristotle, a horismos is a definition: it determines what something is by specifying its boundaries. Configuration does exactly this: it defines and delimits how the system behaves |
| L4 — Reflexive | Horismos delimits: it defines the bounds within which the system operates. The name is itself a delimitation — it draws the boundary around the configuration concern precisely, neither too broad nor too narrow |

---

### Exousia (ἐξουσία)

| Property | Value |
|----------|-------|
| Pronunciation | ex-oo-SEE-ah |
| Directory | mouseion/crates/exousia/ |
| Primary responsibility | Manages household user identities, issues and validates JWT tokens and API keys, and enforces authorization for all protected operations |
| Interface boundary | Exposes an authentication API and an authorization check API; manages the user registry |
| Calls | Horismos (for JWT secrets and session configuration) |
| Called by | Paroche (to authorize streaming), Aitesis (for request limits), Zetesis (for indexer access authorization), all subsystems that expose protected endpoints |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Handles login, token issuance, and access control — who can stream, who can submit requests, who can change configuration |
| L2 — Structural | The gate layer: other subsystems delegate access decisions to Exousia rather than implementing their own. It is the single point of authority |
| L3 — Philosophical | ἐξουσία is authority, power, the right to act — from ἐξ- (out) + οὐσία (being, essence). It names the capacity that flows from proper standing, the legitimate power to do. This subsystem does not just check credentials — it is the site of authority in the system |
| L4 — Reflexive | Exousia authorizes: it is itself authorized to authorize. The name does not merely describe the function — it claims the authority that the subsystem holds |

---

### Syndesmos (σύνδεσμος)

| Property | Value |
|----------|-------|
| Pronunciation | syn-DES-mos |
| Directory | mouseion/crates/syndesmos/ |
| Primary responsibility | Connects Harmonia to external API services — Plex (library sync, collection management, viewing stats), Last.fm (scrobbling, artist metadata), and Tidal (discovery, want-list sync) |
| Interface boundary | Exposes an outbound integration API; manages external service credentials, rate limits, and retry logic |
| Calls | Horismos (for external API credentials), Epignosis (to supply Last.fm artist metadata), Episkope (to sync Tidal want-list) |
| Called by | Episkope (for Tidal discovery), Epignosis (for Last.fm data), Taxis (to notify Plex on import) |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Speaks to Plex, Last.fm, and Tidal on Harmonia's behalf — scrobbling, library notifications, collection sync, discovery data |
| L2 — Structural | The outward boundary: all external API traffic flows through Syndesmos. It holds the external connections so that no other subsystem needs to manage external credentials or retry logic |
| L3 — Philosophical | σύνδεσμος is a connecting bond, a link, a ligament — from συν- (together) + δέω (to bind). It names the thing that holds disparate parts in connection. Syndesmos is the ligament between Harmonia and the external services it cannot absorb |
| L4 — Reflexive | Syndesmos connects: it is the bond between what Harmonia is and what exists outside it. The name binds together the concept of connection and the thing that connects |

---

### Syndesis (σύνδεσις)

| Property | Value |
|----------|-------|
| Pronunciation | syn-DEH-sis |
| Directory | crates/syndesis/ |
| Primary responsibility | QUIC streaming protocol for native client audio transport — server↔renderer FLAC delivery, multi-room clock sync, renderer discovery via mDNS, and renderer pairing |
| Interface boundary | Exposes renderer pairing, stream initiation, and clock sync APIs; operates parallel to Paroche on the same port via QUIC/TCP multiplexing |
| Calls | Horismos (for jitter buffer configuration and stream limits), Exousia (to validate renderer API keys) |
| Called by | harmonia-host (to initialize the streaming service at startup), Akroasis native client (to establish renderer sessions) |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Streams FLAC audio frames from server to renderers over QUIC — clock-synchronized playback across multi-room setups, with mDNS discovery and pairing for native clients |
| L2 — Structural | The native transport layer alongside Paroche: where Paroche serves HTTP clients, Syndesis binds server to native renderers for low-latency, clock-synchronized audio. Both serve the organized library outward — Paroche by provision, Syndesis by binding |
| L3 — Philosophical | σύνδεσις is a binding-together, a joining — from συν- (together) + δέω (to bind). Where Syndesmos names the ligament to external services, Syndesis names the binding between server and renderer — the thread that holds synchronized playback together across the network. The distinction matters: Syndesmos connects Harmonia to what is outside it; Syndesis binds the parts of Harmonia's serving act into a single synchronized whole |
| L4 — Reflexive | Syndesis binds: it is the binding of server to renderer, of clock to clock, of room to room. The protocol is itself a binding — QUIC streams and DATAGRAM heartbeats are the act of syndesis made technical. To name this protocol Syndesis is to name the essential nature of what it does |

---

### Aggelia (ἀγγελία)

| Property | Value |
|----------|-------|
| Pronunciation | an-geh-LEE-ah |
| Directory | within crates/harmonia-common/src/aggelia/ |
| Primary responsibility | Carries internal event announcements between subsystems — past-tense facts about what has occurred within Harmonia |
| Interface boundary | Exposes the `HarmoniaEvent` enum and channel handle types; subscribers receive `broadcast::Receiver<HarmoniaEvent>` distributed by harmonia-host at startup |
| Calls | None — Aggelia carries messages, it does not call subsystems |
| Called by | All subsystems that emit or subscribe to events |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | The internal announcement system — events like "import complete," "quality upgrade triggered," "download progressed." Fire-and-forget. No subsystem waits for Aggelia; it announces and moves on |
| L2 — Structural | The announcement layer between subsystems: where Syndesmos is the bond to external services, Aggelia is the announcement within the system itself. It has no control flow — only facts. It is the nervous system, not the skeleton |
| L3 — Philosophical | ἀγγελία is message, tidings, announcement — the noun form of ἀγγέλλω (to bring tidings). In ancient Greek, Angelia was the personified spirit of messages and proclamations. Unlike σύνδεσμος (the bond/connector), aggelia is the message itself, the act of announcing what has occurred. Every event the bus carries is itself an aggelia — the name describes both the system and every item within it |
| L4 — Reflexive | Aggelia announces: it is the system's capacity for announcing. The name describes the bus and every event it carries simultaneously. To name the bus Aggelia is already to have grasped that every item on the bus is a message — an announcement of something that happened |

---

## Utility Crates (Non-Greek)

Some crates in the Harmonia workspace serve infrastructure roles that fall outside the Greek subsystem naming system. These are shared mechanisms, not domain subsystems.

### harmonia-db

| Property | Value |
|----------|-------|
| Name origin | (utility — no Greek name) |
| Directory | crates/harmonia-db/ |
| Primary responsibility | SQLite database layer — connection pool, schema migrations, and the repository pattern base used by all subsystems that persist state |
| Interface boundary | Provides `DbPool`, migration runner, and generic repository traits; imported by subsystems, never the reverse |
| Calls | Nothing — harmonia-db is a leaf dependency |
| Called by | All stateful subsystems (Episkope, Epignosis, Taxis, Aitesis, Exousia, Syndesmos, Syndesis, and others) |

harmonia-db is not a subsystem — it does not own a domain concern. It is shared infrastructure: the floor that other subsystems stand on when they need to persist state. It is not named Greek because it is not a domain entity; it is a mechanism.

---

## Front-End Domains (Akroasis)

### Prosopon (πρόσωπον)

| Property | Value |
|----------|-------|
| Pronunciation | PROS-oh-pon |
| Directory | akroasis/prosopon/ |
| Primary responsibility | The playback interface — now-playing display, transport controls, queue management, and audio engine interaction |
| Interface boundary | Interacts with the platform audio engine (Rust core); exposes playback state to other Akroasis domains |
| Calls | Theoria (to navigate to albums/tracks), Akroasis audio core (for playback control) |
| Called by | Theoria (to initiate playback), Boulesis (to show what's queued) |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | The now-playing screen — shows what's playing, controls playback, manages the queue |
| L2 — Structural | The face of Akroasis: where Theoria surveys the collection, Prosopon presents the moment of listening. It is what you see when music is playing |
| L3 — Philosophical | πρόσωπον is the face, the mask, the persona — from πρός (toward) + ὤψ (face, eye). In Greek theater it is the mask that gives character to the actor; in philosophy it is the individual as presented. Prosopon is the face that Akroasis turns toward the listener at the moment of listening |
| L4 — Reflexive | Prosopon faces: it is the interface that faces you when you listen, and you face it in return. The now-playing screen is literally a prosopon — the face of the listening act |

---

### Theoria (θεωρία)

| Property | Value |
|----------|-------|
| Pronunciation | theh-oh-REE-ah |
| Directory | akroasis/theoria/ |
| Primary responsibility | Library browsing and search — presents the collection organized by artist, album, genre, and other facets; handles search across all media types |
| Interface boundary | Reads collection state from Mouseion; initiates playback through Prosopon |
| Calls | Prosopon (to initiate playback) |
| Called by | Heuresis (to navigate to discovered items), Boulesis (to browse before requesting) |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Browse your collection — artists, albums, genres, books, podcasts — and search across everything |
| L2 — Structural | The survey layer: where Prosopon attends to what's playing now, Theoria surveys what is available. It is the library seen from a distance |
| L3 — Philosophical | θεωρία is contemplative beholding, the seeing that involves the whole mind — from θεάομαι (to behold, to be a spectator). Aristotle distinguishes theoria from praxis: it is the knowing that consists in looking rather than doing. A library browser is precisely theoria — you look at the collection, behold what's there |
| L4 — Reflexive | Theoria beholds: it presents the collection to the user's contemplation. The library browser is literally a beholding — you see what exists before choosing what to hear |

---

### Diatheesis (διάθεσις)

| Property | Value |
|----------|-------|
| Pronunciation | dee-AH-theh-sis |
| Directory | akroasis/diatheesis/ |
| Primary responsibility | User preferences, playback configuration, server connection settings, and Akroasis-specific defaults |
| Interface boundary | Reads and writes user configuration; surfaces server configuration from Horismos where applicable |
| Calls | None from other domains — settings are read by all |
| Called by | All Akroasis domains (for user preferences affecting their behavior) |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Settings — audio output device, ReplayGain mode, gapless playback, server URL, language preferences |
| L2 — Structural | The dispositional layer: how the user has arranged Akroasis to suit their preferences. Diatheesis is the characterization of this user's listening context |
| L3 — Philosophical | διάθεσις is arrangement, disposition, condition — from διά- (through) + τίθημι (to place). In Aristotle, a diatheesis is a disposition of the soul, a characteristic arrangement that determines how one responds. Settings are a diatheesis: they arrange the application's character to match the user's |
| L4 — Reflexive | Diatheesis arranges: it is the dispositional arrangement of the player to the user's character. The settings domain configures how Akroasis is disposed toward this particular listener |

---

### Boulesis (βούλησις)

| Property | Value |
|----------|-------|
| Pronunciation | boo-LAY-sis |
| Directory | akroasis/boulesis/ |
| Primary responsibility | Household member request submission — search for media to request, submit requests, track status, view request history |
| Interface boundary | Submits to and reads from the backend Aitesis subsystem |
| Calls | Theoria (to browse before requesting), Aitesis backend (to submit and track requests) |
| Called by | None — Boulesis is user-initiated |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Submit a request for media you want — search for it, request it, see when it arrives |
| L2 — Structural | The expression layer: where Aitesis (backend) receives and processes requests, Boulesis is where the user forms and submits them. Together they constitute the full request arc |
| L3 — Philosophical | βούλησις is rational, deliberate desire — the wanting that involves will and judgment, as Aristotle distinguishes it from mere appetite (ἐπιθυμία). A user who browses, selects, and requests is exercising boulesis: they have deliberated and chosen |
| L4 — Reflexive | Boulesis wills: it is the domain where the user's deliberate desire takes form and is submitted. The name is the right word for what happens here — not impulse, but considered wanting |

---

### Heuresis (εὕρεσις)

| Property | Value |
|----------|-------|
| Pronunciation | hew-REH-sis |
| Directory | akroasis/heuresis/ |
| Primary responsibility | Discovery and exploration — surfaces recommendations, related artists, similar albums, and unplayed items from the library |
| Interface boundary | Reads collection and listening history from Mouseion; navigates to Theoria for confirmed items |
| Calls | Theoria (to navigate to discovered items), Boulesis (to request discovered but unowned items) |
| Called by | None — Heuresis is user-initiated |

### Layer Test

| Layer | Reading |
|-------|---------|
| L1 — Practical | Discover what to listen to next — recommendations from your library, related artists, new releases in your collection you haven't heard |
| L2 — Structural | The expansion layer: where Theoria surveys what you know you have, Heuresis surfaces what you didn't know was there. It extends the reach of the collection |
| L3 — Philosophical | εὕρεσις is finding, discovery, invention — from εὑρίσκω (to find, to discover). This is the word Archimedes used. Heuresis is not searching (zetesis) but finding — the moment of encounter with something not previously known |
| L4 — Reflexive | Heuresis finds: it surfaces what was present but unattended. The discovery domain is itself a discovery — it makes the latent collection manifest. The name is the moment of finding, applied to the function of generating moments of finding |

---

## Topology

The full topology diagram and dependency graph live in [topology.md](topology.md).

Summary of containment:

- **Harmonia** contains Mouseion and Akroasis
- **Mouseion** contains the 15 backend subsystems above
- **Akroasis** contains the 5 front-end domains above

Dependency direction flows inward-to-outward:

```
Horismos ← (all subsystems read config)
Exousia ← (all protected subsystems check auth)
Episkope → Zetesis → Ergasia → Syntaxis → Taxis → Kritike
                                          Taxis → Epignosis
                                          Taxis → Prostheke
Episkope ← Aitesis
Paroche ← (Akroasis HTTP clients)
Syndesis ← (Akroasis native renderers, QUIC)
Syndesmos ↔ (external: Plex, Last.fm, Tidal)
```

No circular dependencies. Horismos and Exousia are the only leaf-level subsystems (they call nothing within Mouseion).

---

## VISION.md Coverage Verification

Every function from the [VISION.md](../VISION.md) tool replacement map is covered:

| VISION.md Function | Subsystem |
|--------------------|-----------|
| Media lifecycle management (Sonarr/Radarr/Lidarr/Readarr) | Episkope |
| Indexer aggregation, Torznab/Newznab (Prowlarr/Jackett) | Zetesis |
| Torrent downloads (qBittorrent) | Ergasia |
| Archive extraction (Unpackerr) | Ergasia |
| Cloudflare bypass (FlareSolverr) | Zetesis |
| Download orchestration, post-import triggers (Pulsarr/Autopulse) | Syntaxis |
| Audiobook serving (Audiobookshelf) | Paroche + Akroasis |
| Book/comic serving (Kavita) | Paroche + Akroasis |
| Subtitle management (Bazarr) | Prostheke |
| Media transcoding (Tdarr) | Paroche |
| Download queue hygiene (Cleanarr/Decluttarr) | Kritike |
| File integrity verification (Checkrr) | Kritike |
| Auto-cleanup rules (Maintainerr) | Kritike |
| Monitoring rules, tagging (Excludarr/Labelarr) | Episkope + Kritike |
| Quality assessment and upgrades (Quasarr) | Kritike |
| Plex collection management (Kometa) | Syndesmos |
| Plex viewing statistics (Wrapperr) | Syndesmos |
| Media request workflow (Overseerr) | Aitesis + Boulesis |
| Metadata enrichment (MusicBrainz/TMDB/TVDB/Audnexus) | Epignosis |
| Import and organization (rename, file structure) | Taxis |
