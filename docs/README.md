# Harmonia documentation

> Start here. This index lists real documents in this directory. Planned-but-not-written pages live in GitHub issues, not as dangling links.

Agents: for a compressed crate map and technology index, load
[`../_llm/architecture.toml`](../_llm/architecture.toml) and
[`../_llm/decisions.toml`](../_llm/decisions.toml) first.

## Foundation

- [VISION.md](VISION.md): What Harmonia is, what it replaces, and why
- [lexicon.md](lexicon.md): Subsystem names with layer tests
- [PROJECT.md](PROJECT.md): Project overview and phase status
- [LESSONS.md](LESSONS.md): Operational rules from real failures
- [WORKING-AGREEMENT.md](WORKING-AGREEMENT.md): Syn + Cody collaboration protocol
- [../standards/STANDARDS.md](../standards/STANDARDS.md): Code standards (kanon-synced)

## Architecture

- [architecture/subsystems.md](architecture/subsystems.md): Subsystem map, domain ownership, direct-call vs event classification
- [architecture/cargo.md](architecture/cargo.md): Cargo workspace layout + dependency DAG
- [architecture/binary-modes.md](architecture/binary-modes.md): serve / desktop / render / play
- [architecture/communication.md](architecture/communication.md): Aggelia event bus + internal messaging patterns
- [architecture/configuration.md](architecture/configuration.md): figment config cascade + secret overlay
- [architecture/errors.md](architecture/errors.md): snafu one-enum-per-crate convention
- [architecture/auth.md](architecture/auth.md): JWT + API keys + argon2

## Data

- [data/media-schemas.md](data/media-schemas.md): Per-media-type table schemas
- [data/want-release.md](data/want-release.md): Want vs Release concept
- [data/quality-profiles.md](data/quality-profiles.md): Quality profile system
- [data/storage.md](data/storage.md): SQLite WAL + migration strategy
- [data/storage-layout.md](data/storage-layout.md): Filesystem layout for imported media
- [data/entity-registry.md](data/entity-registry.md): Shared entity + UUID cross-reference

## Download and acquisition

- [download/torrent.md](download/torrent.md): librqbit integration
- [download/indexer-protocol.md](download/indexer-protocol.md): Torznab/Newznab direct implementation
- [download/orchestration.md](download/orchestration.md): Queue, post-processing, import pipeline
- [download/cloudflare.md](download/cloudflare.md): Cloudflare bypass architecture
- [download/archive.md](download/archive.md): Archive extraction (zip/rar/7z)
- [download/usenet.md](download/usenet.md): Usenet feasibility and approach
- [download/irc.md](download/irc.md): IRC announce integration
- [download/p3-02-observations.md](download/p3-02-observations.md): Phase 3 librqbit + unrar API notes

## Media lifecycle and metadata

- [media/lifecycle.md](media/lifecycle.md): Per-type lifecycle state machines
- [media/metadata-providers.md](media/metadata-providers.md): Provider strategy + rate limiting
- [media/scanner.md](media/scanner.md): Library scanner design
- [media/import-rename.md](media/import-rename.md): Import and rename pipeline
- [media/music.md](media/music.md): Music-specific design (MusicBrainz, ReplayGain)
- [media/audiobooks.md](media/audiobooks.md): Audiobook-specific design
- [media/news.md](media/news.md): News feed design (RSS/Atom)
- [media/subtitles.md](media/subtitles.md): Subtitle management

## Serving

- [serving/streaming.md](serving/streaming.md): HTTP media streaming + transcoding

## Integrations

- [integrations.md](integrations.md): Third-party client sync (KOSync for KOReader), provider bridges

## Desktop

- [desktop/architecture.md](desktop/architecture.md): Dioxus desktop app (proskenion)

## Deployment

- [nix-deployment.md](nix-deployment.md): Nix-based deployment

## Policy

- [policy/agent-contribution.md](policy/agent-contribution.md): Agent PR and commit rules
- [policy/versioning.md](policy/versioning.md): Versioning policy

Commit conventions (conventional commits, squash merge, branch naming) live in
[`../AGENTS.md`](../AGENTS.md) and [`../CLAUDE.md`](../CLAUDE.md). Git history
policy that was previously duplicated here has been removed per
`standards/AGENT-DOCS.md` §Delete, don't stub.
