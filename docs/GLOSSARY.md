# Glossary

> Single source of truth for shared values. Other docs link here; they do not independently define these values.
> Never define these values independently in another doc.

## Platform Names

| Name | Pronunciation | What It Is |
|------|--------------|------------|
| Harmonia | har-MOH-nee-ah | The unified media platform — backend + player |
| Mouseion | moo-SAY-on | Media management backend (Rust) |
| Aggelia | an-geh-LEE-ah | Carries internal event announcements between subsystems |
| Aitesis | eye-TAY-sis | Manages the household media request workflow — submission, approval, status tracking, and handoff to Episkope |
| Epignosis | ep-ee-GNOH-sis | Enriches media with metadata from external providers and maintains the metadata cache |
| Episkope | ep-ee-sko-PAY | Monitors the state of wanted media and triggers acquisition when gaps are found |
| Ergasia | er-GAH-see-ah | Executes torrent downloads and archive extraction |
| Exousia | ex-oo-SEE-ah | Manages identity, authentication, and authorization for household users |
| Horismos | hor-is-MOS | Owns all system configuration as the single parameterized source of truth |
| Kritike | kree-tee-KAY | Assesses library quality, verifies integrity, and enforces curation rules |
| Paroche | pah-ro-KAY | Serves media to clients via HTTP streaming, OPDS, and transcoding |
| Prostheke | pros-THAY-kay | Manages subtitle acquisition, synchronization, and storage |
| Syndesmos | syn-DES-mos | Connects Harmonia to external API services (Plex, Last.fm, Tidal) |
| Syntaxis | syn-TAK-sis | Coordinates the download queue, priority, and post-processing pipeline |
| Taxis | TAK-sis | Imports and organizes media files into the library structure |
| Zetesis | zay-TAY-sis | Queries indexers for available media using Torznab/Newznab protocols |
| Akroasis | ah-kroh-AH-sis | Media player — Android, Web, Desktop |
| Boulesis | boo-LAY-sis | Household member request submission and tracking |
| Diatheesis | dee-AH-theh-sis | User preferences and server configuration |
| Heuresis | hew-REH-sis | Discovery, exploration, and recommendations |
| Prosopon | PROS-oh-pon | The playback interface — now-playing, controls, queue |
| Theoria | theh-oh-REE-ah | Library browsing and search across the full collection |

## Key Paths

| Purpose | Path |
|---------|------|
| Documentation root | docs/ |
| Documentation index | docs/README.md |
| Naming system | docs/gnomon.md |
| Subsystem registry | docs/naming/registry.md |
| Subsystem topology | docs/naming/topology.md |
| Code standards | docs/STANDARDS.md |
| Dispatch protocol | docs/CLAUDE_CODE.md |
| Operational rules | docs/LESSONS.md |
| Collaboration protocol | docs/WORKING-AGREEMENT.md |
| Policy directory | docs/policy/ |

## Tool Replacement Reference

See [VISION.md](VISION.md) — canonical location for the full replacement map.
