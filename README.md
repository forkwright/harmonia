# Harmonia

> Ἁρμονία (Harmonia) — "the fitting together of disparate parts"

Unified self-hosted media platform. One system, two deployment targets.

## Components

| Component | Path | Stack | Description |
|-----------|------|-------|-------------|
| **Mouseion** | `mouseion/` | .NET 10, C#, Dapper, SQLite/PostgreSQL | Media management backend — movies, TV, music, books, audiobooks, podcasts, manga, comics, news |
| **Akroasis** | `akroasis/` | Kotlin/Compose, React 19/TS, Rust audio core | Multi-platform media player — Android, Web, Desktop (planned) |

## Development

Each component builds independently. See component READMEs for setup:

- [mouseion/README.md](mouseion/README.md) — Backend API (port 7878)
- [akroasis/README.md](akroasis/README.md) — Player clients

## License

GPL-3.0
