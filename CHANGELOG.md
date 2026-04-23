# Changelog

## [0.1.9](https://github.com/forkwright/harmonia/compare/v0.1.8...v0.1.9) (2026-04-22)


### Features

* **epignosis:** add Google Books provider + OL edition-level fetch ([#217](https://github.com/forkwright/harmonia/issues/217)) ([b22f77a](https://github.com/forkwright/harmonia/commit/b22f77ad64534d5a7b836e3642881654d4facbc1))
* **harmonia-convert:** introduce subprocess-based ebook conversion crate ([#219](https://github.com/forkwright/harmonia/issues/219)) ([11f10f0](https://github.com/forkwright/harmonia/commit/11f10f0a3fa636780a8b4f7564235a2fc49ff6bf)), closes [#211](https://github.com/forkwright/harmonia/issues/211)
* **paroche:** KOSync protocol for ebook reading-progress sync ([#220](https://github.com/forkwright/harmonia/issues/220)) ([a34b893](https://github.com/forkwright/harmonia/commit/a34b8937ecfc333612ee46c74c06be9301e0412b))
* **paroche:** vendored foliate-js reader at /read/:book_id ([#218](https://github.com/forkwright/harmonia/issues/218)) ([6a324fe](https://github.com/forkwright/harmonia/commit/6a324fe6c06cca5575dbaf5c5b713e6add8bf343))


### Bug Fixes

* **apotheke:** migration 004 uses INTEGER not BOOLEAN for STRICT tables (closes [#194](https://github.com/forkwright/harmonia/issues/194)) ([#196](https://github.com/forkwright/harmonia/issues/196)) ([b16e8a6](https://github.com/forkwright/harmonia/commit/b16e8a62f6daa4bcca5e6ff55b7b9c20ff2372bd))
* **archon:** sd-notify 0.5 dropped unset_env arg — update callsites ([#202](https://github.com/forkwright/harmonia/issues/202)) ([69c798f](https://github.com/forkwright/harmonia/commit/69c798ffd0b2b813e600aff9209c6c3610b2fe9b))
* **ci:** pre-stage forge-CI memory caps for Phase 05e cutover ([#200](https://github.com/forkwright/harmonia/issues/200)) ([b9fb1bc](https://github.com/forkwright/harmonia/commit/b9fb1bc13949bbb6663991967d230e83b13a7ea3))
* **clippy:** clear 5 pre-existing too-many-args + unfulfilled expect errors ([#204](https://github.com/forkwright/harmonia/issues/204)) ([d2a85d3](https://github.com/forkwright/harmonia/commit/d2a85d3bd05ea05ed9be0b4ca34658d835d4b74c))
* **clippy:** resolve .get(0) and unnecessary_fallible_conversions warnings ([2f0290c](https://github.com/forkwright/harmonia/commit/2f0290cfb7c875a6838bd580bdb441a4badc0a29))
* **epignosis/openlibrary:** lowercase "limit" param; Solr ignores uppercase ([#216](https://github.com/forkwright/harmonia/issues/216)) ([4ff2289](https://github.com/forkwright/harmonia/commit/4ff2289886408851b5598541a0914b22fb853fb0))
* **komide:** validate_url uses url::Url parse instead of prefix match (closes [#203](https://github.com/forkwright/harmonia/issues/203)) ([#205](https://github.com/forkwright/harmonia/issues/205)) ([48a7f9a](https://github.com/forkwright/harmonia/commit/48a7f9affdc3025cf9d8c091b49d0f3a2841186c))
* **lint:** add #[non_exhaustive] to 44 public enums + wildcard match arms ([#207](https://github.com/forkwright/harmonia/issues/207)) ([1384523](https://github.com/forkwright/harmonia/commit/13845232d9bf320b5654b596e5d8b948273e1fb6))
* **lint:** clear 16 RUST/unwrap warnings (per-site decisions documented in body) ([#209](https://github.com/forkwright/harmonia/issues/209)) ([0c0f16f](https://github.com/forkwright/harmonia/commit/0c0f16f93c179593f619492435e2a21aad0bce99))
* **lint:** replace 3 direct indexing sites with .get() + None handling ([#208](https://github.com/forkwright/harmonia/issues/208)) ([077f542](https://github.com/forkwright/harmonia/commit/077f54279cfb2842808e20b7e7fd5fd7b904a8d0))

## [0.1.8](https://github.com/forkwright/harmonia/compare/v0.1.7...v0.1.8) (2026-04-15)


### Bug Fixes

* **ci:** fix gate-attestation job name and fetch base branch ([#190](https://github.com/forkwright/harmonia/issues/190)) ([4df8454](https://github.com/forkwright/harmonia/commit/4df84546c37cbdaa7abd756c551238782bbcef51))
* **sql:** add IF NOT EXISTS and STRICT to table definitions ([#192](https://github.com/forkwright/harmonia/issues/192)) ([f51917a](https://github.com/forkwright/harmonia/commit/f51917a3f7c15eb721d1b75698f0131199dd8143))

## [0.1.7](https://github.com/forkwright/harmonia/compare/v0.1.6...v0.1.7) (2026-04-13)


### Features

* **archon:** migrate subcommand for legacy library conversion ([#163](https://github.com/forkwright/harmonia/issues/163)) ([#185](https://github.com/forkwright/harmonia/issues/185)) ([4dd3d0c](https://github.com/forkwright/harmonia/commit/4dd3d0cb14a2af25f7cfe7a804263212a4448b35))
* **epignosis:** Audnexus enrichment for audiobook metadata ([#164](https://github.com/forkwright/harmonia/issues/164)) ([#183](https://github.com/forkwright/harmonia/issues/183)) ([a5716eb](https://github.com/forkwright/harmonia/commit/a5716ebe5c006c17efa871c4d42b580cca575271))
* **kathodos:** artist alias symlink management ([#162](https://github.com/forkwright/harmonia/issues/162)) ([#179](https://github.com/forkwright/harmonia/issues/179)) ([9f2d2f7](https://github.com/forkwright/harmonia/commit/9f2d2f78803b53b4705fc003c8090dfb3e519978))
* **kathodos:** canonical path templates for all media types ([#158](https://github.com/forkwright/harmonia/issues/158), [#159](https://github.com/forkwright/harmonia/issues/159)) ([#184](https://github.com/forkwright/harmonia/issues/184)) ([94bfb19](https://github.com/forkwright/harmonia/commit/94bfb19076a8026ba59e49ba2d07dd1625e426e2))
* **kathodos:** smart path sanitization for canonical storage ([#160](https://github.com/forkwright/harmonia/issues/160)) ([#182](https://github.com/forkwright/harmonia/issues/182)) ([8314d45](https://github.com/forkwright/harmonia/commit/8314d4590d26900e251a33197f53311e40f6a23e))
* **kathodos:** TOML sidecar reader/writer for all media types ([#161](https://github.com/forkwright/harmonia/issues/161)) ([#177](https://github.com/forkwright/harmonia/issues/177)) ([ff41c74](https://github.com/forkwright/harmonia/commit/ff41c746fae8f65aca43927bd69733b89bb9f1a4))
* **kritike:** format quality scoring for ebooks and audiobooks ([#165](https://github.com/forkwright/harmonia/issues/165)) ([#180](https://github.com/forkwright/harmonia/issues/180)) ([4ba4902](https://github.com/forkwright/harmonia/commit/4ba490290b9c66c217763646863a23a64226cda8))

## [0.1.6](https://github.com/forkwright/harmonia/compare/v0.1.5...v0.1.6) (2026-04-13)


### Bug Fixes

* **ops:** upgrade to AGPL-3.0, add AI training clause and .aiignore ([#139](https://github.com/forkwright/harmonia/issues/139)) ([#172](https://github.com/forkwright/harmonia/issues/172)) ([e782981](https://github.com/forkwright/harmonia/commit/e782981184557a69c63e35ade5a9b74f1f5a87c6))
* restore workspace compilation broken by kanon lint --fix ([#168](https://github.com/forkwright/harmonia/issues/168)) ([43375aa](https://github.com/forkwright/harmonia/commit/43375aa829e1e3a8582a3ce184b041891e202f7b))

## [0.1.5](https://github.com/forkwright/harmonia/compare/v0.1.4...v0.1.5) (2026-04-04)


### Features

* **aitesis:** request management (P3-05) ([#87](https://github.com/forkwright/harmonia/issues/87)) ([217b523](https://github.com/forkwright/harmonia/commit/217b523303e78e89ec48659084ccd5e94a067419))
* **akouo-core:** extract audio engine into workspace crate ([#121](https://github.com/forkwright/harmonia/issues/121)) ([#125](https://github.com/forkwright/harmonia/issues/125)) ([f378e1c](https://github.com/forkwright/harmonia/commit/f378e1c4999eea9ba67b5a4dd29b3afd13a35af7))
* **akroasis-core:** cpal output backend, format negotiation, resampler ([#39](https://github.com/forkwright/harmonia/issues/39)) ([1fc3e61](https://github.com/forkwright/harmonia/commit/1fc3e613f08436597f127a8c84bb97a87b054dc6))
* **akroasis-core:** DSP stages 5–7 — compressor, convolution, volume+dither ([#35](https://github.com/forkwright/harmonia/issues/35)) ([329fb55](https://github.com/forkwright/harmonia/commit/329fb55f2e8b72fbf1a5aae2880a96ae7a8349e6))
* **akroasis-core:** gapless playback scheduler, crossfade, and codec delay trimming ([#40](https://github.com/forkwright/harmonia/issues/40)) ([d69d522](https://github.com/forkwright/harmonia/commit/d69d522ca5c832ef75ca67d9df01ac3d6a5c5b9d))
* **akroasis-core:** scaffold module structure, core types, ring buffer ([#33](https://github.com/forkwright/harmonia/issues/33)) ([9f17877](https://github.com/forkwright/harmonia/commit/9f178777e04f80f80de30f9ac80627193e18f80d))
* **akroasis-core:** Symphonia decode pipeline ([#37](https://github.com/forkwright/harmonia/issues/37)) ([14d2809](https://github.com/forkwright/harmonia/commit/14d2809e63315ab09663d58065e511b7e4d67d8c))
* **akroasis:** add DSP controls UI with crossfeed presets, ReplayGain modes, and output device selector ([#98](https://github.com/forkwright/harmonia/issues/98)) ([7be6105](https://github.com/forkwright/harmonia/commit/7be610568da52509d4b52b05902bfe7002beb04a))
* **akroasis:** Android audio playback — pipeline, auth, queue, media session ([7a4d6f5](https://github.com/forkwright/harmonia/commit/7a4d6f54a5d73a2c9961f219b9b7a77a36809f88))
* **akroasis:** Android audiobook playback, ebook reader, and CI releases ([93aa8dc](https://github.com/forkwright/harmonia/commit/93aa8dcddf2dc67002113f1964aad7e2b66b305c))
* **akroasis:** Android Phase 2 — UI scaffolding, search, audio intelligence ([c493ec3](https://github.com/forkwright/harmonia/commit/c493ec30914d57013f7ee98535c01abe166a3f68))
* **akroasis:** audio DSP, listening DNA, podcast management, and nav redesign ([b19728c](https://github.com/forkwright/harmonia/commit/b19728c9215a7df8dcb23c0b9ea0ce33633ac9fc))
* **akroasis:** audiobook support — library, player, chapters, progress tracking ([154c932](https://github.com/forkwright/harmonia/commit/154c932c9f4a1ffbe8cb1e45d46b9345a04f8b2d))
* **akroasis:** design coherence — warm parchment, serif headings, login defaults ([423ba85](https://github.com/forkwright/harmonia/commit/423ba852adf1e7e97dd79017e621ce01cbf397c0))
* **akroasis:** design system, library browsing, adaptive experience, media-type players ([6e112f8](https://github.com/forkwright/harmonia/commit/6e112f8d4eb998c5cacea29c1d0b903fedfdb423))
* **akroasis:** DSP stages 1–4 — skip silence, parametric EQ, crossfeed, ReplayGain ([#38](https://github.com/forkwright/harmonia/issues/38)) ([41e0425](https://github.com/forkwright/harmonia/commit/41e0425fa23e412886f638cef80ce63e2dee9fa3))
* **akroasis:** engine wiring, PlayQueue, and harmonia CLI (P1-08) ([#41](https://github.com/forkwright/harmonia/issues/41)) ([0a2ab99](https://github.com/forkwright/harmonia/commit/0a2ab99fa03566af47f57e1dbec3fb4de19a1c9a))
* **akroasis:** integration cycle, QA, and CI workflows ([caaacec](https://github.com/forkwright/harmonia/commit/caaacecd0cb3d19b3c5eed1cff0cc3fe46727bec))
* **akroasis:** Opus FFI decoder bridge and WavPack skeleton (P1-03) ([#36](https://github.com/forkwright/harmonia/issues/36)) ([205c3b0](https://github.com/forkwright/harmonia/commit/205c3b03c67f40ee3783c4a8410730822c356379))
* **akroasis:** playback progress tracking and session management ([d772139](https://github.com/forkwright/harmonia/commit/d7721398838d488573ffb175f3f31c11655af01b))
* **akroasis:** sleep timer, bookmarks, lyrics, EQ, Android Auto, artwork zoom ([190527b](https://github.com/forkwright/harmonia/commit/190527b2d2284592d28914cc0c1b23d9be9e5743))
* **akroasis:** test coverage to 80% and voice search integration ([90b222e](https://github.com/forkwright/harmonia/commit/90b222e159bda18629e1a9cd019467eccd03ed47))
* **akroasis:** voice search, A/B comparison, accessibility, settings, and test coverage ([ce8fafa](https://github.com/forkwright/harmonia/commit/ce8fafae845b7414f39ad967cb76948e5185dd77))
* **akroasis:** web auth, discovery, cross-device sync, global search, AutoEQ ([1277f3c](https://github.com/forkwright/harmonia/commit/1277f3c093bbf4cabf4eebae006446bb69d152e9))
* **akroasis:** web bug fixes — auth, API alignment, theme unification ([12758e6](https://github.com/forkwright/harmonia/commit/12758e63347da7b3eca32745bc94b2dffcb7bd6e))
* **akroasis:** web MVP — gapless playback, queue, keyboard shortcuts, PWA ([2cbba2e](https://github.com/forkwright/harmonia/commit/2cbba2ed618a3fd399562696fcbf46db09119dac))
* **akroasis:** web player foundation — mock API server, library browsing, bit-perfect audio ([8563435](https://github.com/forkwright/harmonia/commit/856343509bc32736f4109d4253b9d369843f878a))
* **akroasis:** web UI overhaul — player, library, navigation, design system, playback engine ([dee4c1b](https://github.com/forkwright/harmonia/commit/dee4c1b1816cf6400611e7284824da2b276671c4))
* album art endpoint, sort controls, playlist tracks API ([6a6216a](https://github.com/forkwright/harmonia/commit/6a6216a8b220cdc12465245e372f986a78c48ef6))
* cover art, favorites, playlists, library UI, and artist stats ([c8f6133](https://github.com/forkwright/harmonia/commit/c8f613371f030d36825a5a28a02b8ebbeec2ca42))
* **desktop:** audiobook player with chapters and bookmarks (P3-13) ([#82](https://github.com/forkwright/harmonia/issues/82)) ([26129ee](https://github.com/forkwright/harmonia/commit/26129ee293eb74361ab1d982fa714cb3fce1cc20))
* **desktop:** EQ panel and DSP controls with AutoEQ (P3-12) ([#71](https://github.com/forkwright/harmonia/issues/71)) ([e4b659a](https://github.com/forkwright/harmonia/commit/e4b659a1eb0bbb54f40546723ed8f3c1e74a75c1))
* **desktop:** library browser — album/artist/track views (P3-10) ([#70](https://github.com/forkwright/harmonia/issues/70)) ([487e8ea](https://github.com/forkwright/harmonia/commit/487e8ead30007c3df5074d9228a98607185bfd7a))
* **desktop:** media management UI for all 8 types (P3-15) ([#88](https://github.com/forkwright/harmonia/issues/88)) ([ec96517](https://github.com/forkwright/harmonia/commit/ec96517e5af2dc172e6d579e8752df9b5ed8eddc))
* **desktop:** MPRIS, system tray, and OS integration (P3-16) ([#92](https://github.com/forkwright/harmonia/issues/92)) ([2fac949](https://github.com/forkwright/harmonia/commit/2fac9493113f90d1988281b9ab0da9c0f9ba83a4))
* **desktop:** now playing with playback controls and queue (P3-11) ([#84](https://github.com/forkwright/harmonia/issues/84)) ([93fdf14](https://github.com/forkwright/harmonia/commit/93fdf14247e4d64205f240c95be6350764157a32))
* **desktop:** podcast player with subscriptions and episodes (P3-14) ([#81](https://github.com/forkwright/harmonia/issues/81)) ([917ba1f](https://github.com/forkwright/harmonia/commit/917ba1faece379b84a07fe56ec92633e2ba43fc0))
* **desktop:** Tauri 2 scaffold with React 19 (P3-09) ([1c4b6cb](https://github.com/forkwright/harmonia/commit/1c4b6cbf4c7621d631f698dc51967a882d82277c))
* **epignosis:** metadata enrichment (P2-06) ([#46](https://github.com/forkwright/harmonia/issues/46)) ([516ec30](https://github.com/forkwright/harmonia/commit/516ec307febe7efb834223615dd7a4563b8118b1))
* **ergasia:** download execution and archive extraction (P3-02) ([#69](https://github.com/forkwright/harmonia/issues/69)) ([93582e1](https://github.com/forkwright/harmonia/commit/93582e11e975a37d1485c2aa6c046a25c3ae7412))
* error logging and diagnostics — IndexedDB, server persistence, client log API ([5e25453](https://github.com/forkwright/harmonia/commit/5e25453a695bdab93d3475a3e1a1f56f80ace318))
* **exousia:** authentication and authorization (P2-04) ([#48](https://github.com/forkwright/harmonia/issues/48)) ([358d9a9](https://github.com/forkwright/harmonia/commit/358d9a9c44000d8ee50a85f7bc3fd0ea97bfdda8))
* **harmonia-db:** play history, scrobble tracking, and listening analytics (P2-14) ([#45](https://github.com/forkwright/harmonia/issues/45)) ([88f0e93](https://github.com/forkwright/harmonia/commit/88f0e93864ad7b847e417d88ecddc1cb4c5b9029))
* **harmonia-db:** SQLite database layer with dual-pool WAL (P2-03) ([#44](https://github.com/forkwright/harmonia/issues/44)) ([32a53f9](https://github.com/forkwright/harmonia/commit/32a53f9560133f8a2b2a84479a8fefbdf9effbfc))
* **harmonia-host:** add render subcommand with local DSP and status reporting ([#127](https://github.com/forkwright/harmonia/issues/127)) ([f09f84e](https://github.com/forkwright/harmonia/commit/f09f84e5fd1976209fcb45b0409946911e9962d1))
* **harmonia-host:** serve mode wiring (P2-12) ([#56](https://github.com/forkwright/harmonia/issues/56)) ([c61d57d](https://github.com/forkwright/harmonia/commit/c61d57d5210032064e66712693c31a357227d29b))
* **harmonia-host:** wire acquisition subsystems into startup/shutdown (P101) ([#100](https://github.com/forkwright/harmonia/issues/100)) ([e82418b](https://github.com/forkwright/harmonia/commit/e82418bd52dd7ad7799a36ad12df11a423879444))
* **horismos:** configuration loading and validation (P2-02) ([#43](https://github.com/forkwright/harmonia/issues/43)) ([ecf6ad5](https://github.com/forkwright/harmonia/commit/ecf6ad532db1b92eff314286706c8ad3be88be51))
* **kritike:** quality profiles and library health (P2-07) ([#47](https://github.com/forkwright/harmonia/issues/47)) ([faa03ed](https://github.com/forkwright/harmonia/commit/faa03ed09a13e3dd5a81168d3ea10660090c4e53))
* **mouseion:** API quality, quality detection, and advanced file import pipeline ([b27037d](https://github.com/forkwright/harmonia/commit/b27037d8f50a86fbd3a7724126fefa95cdb30e14))
* **mouseion:** auth system, import workflows, smart playlists, and build fixes ([c77d909](https://github.com/forkwright/harmonia/commit/c77d909a570bfc4fe5feea0111bdca4fde520ddf))
* **mouseion:** database foundation — MediaItems, SignalR, DI wiring, CI ([33e2bbd](https://github.com/forkwright/harmonia/commit/33e2bbd6b6a6dea320de2241a0c43b9b8dfc6ef6))
* **mouseion:** Docker containerization and production hardening ([f29859a](https://github.com/forkwright/harmonia/commit/f29859a772a03e51dc6f65679e5bdf3e31294209))
* **mouseion:** import wizard, user permissions, acquisition orchestration ([8644b46](https://github.com/forkwright/harmonia/commit/8644b46f12c997481511263bb69e0cf0508e56f8))
* **mouseion:** media scanners, streaming endpoint, and search improvements ([26470f6](https://github.com/forkwright/harmonia/commit/26470f61abff10ab4cba1cd1d86533c8069dd642))
* **mouseion:** music and movie APIs — MusicBrainz, TMDb, file scanning ([2111bac](https://github.com/forkwright/harmonia/commit/2111bac89abaf24e3f6eb5a48695e0ea7c2889ac))
* **mouseion:** news/RSS, manga, comics, health checks, notifications, and validation ([e655d8a](https://github.com/forkwright/harmonia/commit/e655d8a27502a412f4aac8fa9086bdd6ca99520b))
* **mouseion:** OIDC auth, tech debt cleanup, and dependency updates ([b881e11](https://github.com/forkwright/harmonia/commit/b881e111550adf1986f4bb11bb290cb8d5e67684))
* **mouseion:** OpenSubtitles, rate limiting, audiobook chapters, progress tracking, RFC 7807 ([58861c3](https://github.com/forkwright/harmonia/commit/58861c3e7861ec0bd2a30a037920ff47ca89007c))
* **mouseion:** podcast and news feed subscription (P2-11) ([#50](https://github.com/forkwright/harmonia/issues/50)) ([92a97ff](https://github.com/forkwright/harmonia/commit/92a97ff1a4eba397fac357ee1c0a5eb4853d0990))
* **mouseion:** port core infrastructure from Radarr — DI, HTTP, serialization, disk, crypto ([71f99e1](https://github.com/forkwright/harmonia/commit/71f99e1db9152780e13987a8f3cda0d54eb4809c))
* **mouseion:** SIGHUP config reload (P2-15) ([#55](https://github.com/forkwright/harmonia/issues/55)) ([6f6aed1](https://github.com/forkwright/harmonia/commit/6f6aed1cf7c8a15936ed5c81da21e99863979356))
* **mouseion:** TV, podcasts, notifications, download clients, and archive migration ([80acbd9](https://github.com/forkwright/harmonia/commit/80acbd9000e65ce505781c95f0e9676fc278a5b9))
* **mouseion:** TVDB v4 integration, bulk operations, LoggerMessage, and OpenTelemetry ([73bc122](https://github.com/forkwright/harmonia/commit/73bc12229ba365228a8e03189bdfd3452ad104ae))
* **mouseion:** webhook ingestion, OPDS catalog, smart lists, and analytics ([be67f18](https://github.com/forkwright/harmonia/commit/be67f18b8614110d35e656b48dd4fd4d79f856e5))
* **nix:** NixOS renderer module with DAC HAT overlays and aarch64 cross-compilation ([#130](https://github.com/forkwright/harmonia/issues/130)) ([2c45336](https://github.com/forkwright/harmonia/commit/2c45336aa09f57bd401b74afb5b81d4c86c7c0d1))
* NixOS module for declarative deployment (P2-13) ([#57](https://github.com/forkwright/harmonia/issues/57)) ([702237a](https://github.com/forkwright/harmonia/commit/702237af995bb8d902a3e6af74a28db60ead8ca0))
* **paroche:** acquisition API endpoints (P102) ([#101](https://github.com/forkwright/harmonia/issues/101)) ([931e18c](https://github.com/forkwright/harmonia/commit/931e18c5b97628f44fd6507878090cb1dc22224e))
* **paroche:** core HTTP API (P2-08) ([#51](https://github.com/forkwright/harmonia/issues/51)) ([f83361b](https://github.com/forkwright/harmonia/commit/f83361bcd385a8a0a10181386d052ee892d1782a))
* **paroche:** OPDS 2.0 catalog (P2-10) ([#52](https://github.com/forkwright/harmonia/issues/52)) ([f0f89b6](https://github.com/forkwright/harmonia/commit/f0f89b6ef6a12930fa5c919d69af8c097b93e141))
* **paroche:** OpenSubsonic API (P2-09) ([#54](https://github.com/forkwright/harmonia/issues/54)) ([703ee54](https://github.com/forkwright/harmonia/commit/703ee5439fd75deba913b30680f076513b739a2c))
* **prostheke:** subtitle management (P3-07) ([#91](https://github.com/forkwright/harmonia/issues/91)) ([76c1885](https://github.com/forkwright/harmonia/commit/76c1885935e39748ce888ee7cb94e6f9f0cf58d4))
* **syndesis:** mDNS discovery, pairing protocol, and renderer registry ([#128](https://github.com/forkwright/harmonia/issues/128)) ([31fe086](https://github.com/forkwright/harmonia/commit/31fe08684bb288b80555faa06e3311a27272af9f))
* **syndesis:** multi-room zone grouping with &lt;=5ms clock sync ([#129](https://github.com/forkwright/harmonia/issues/129)) ([f46356b](https://github.com/forkwright/harmonia/commit/f46356b3660c3d808e3071dfa13082fb710e6dd8))
* **syndesis:** QUIC streaming protocol with clock sync and jitter buffer ([#126](https://github.com/forkwright/harmonia/issues/126)) ([ee925c6](https://github.com/forkwright/harmonia/commit/ee925c6f9b9834ba000bf4c4f0efe162556ed8b9))
* **syndesmos:** external service integration crate (P3-06) ([#83](https://github.com/forkwright/harmonia/issues/83)) ([459cf97](https://github.com/forkwright/harmonia/commit/459cf971086ee168f80dde8d2deab6934e92bcc8))
* **syntaxis:** queue orchestration and post-processing (P3-03) ([#90](https://github.com/forkwright/harmonia/issues/90)) ([3766e59](https://github.com/forkwright/harmonia/commit/3766e5912da6370a20cebed26ed55fc4bdc44d68))
* **taxis:** library scanner and import pipeline (P2-05) ([#49](https://github.com/forkwright/harmonia/issues/49)) ([2914dc4](https://github.com/forkwright/harmonia/commit/2914dc417e0b0365ef19a5b1c37446272523c0a1))
* **theatron:** scaffold Dioxus desktop app (phase 0 of [#120](https://github.com/forkwright/harmonia/issues/120)) ([#122](https://github.com/forkwright/harmonia/issues/122)) ([520c8c1](https://github.com/forkwright/harmonia/commit/520c8c1c7fc0553d5380097264ca92d055d6d4dc))
* workspace scaffold and harmonia-common crate (P2-01) ([#42](https://github.com/forkwright/harmonia/issues/42)) ([6aa8642](https://github.com/forkwright/harmonia/commit/6aa864239e05efa2a5293bf77b2d263ba67c95fa))
* **zetesis:** indexer protocol and search routing (P3-01) ([#59](https://github.com/forkwright/harmonia/issues/59)) ([2b7a4fe](https://github.com/forkwright/harmonia/commit/2b7a4fecd83b7032e3ecdafaf7b5a6b1b7781c38))


### Bug Fixes

* add [graph] section to deny.toml for cargo-deny 0.19 compatibility ([f0d0811](https://github.com/forkwright/harmonia/commit/f0d0811d38bb85d14683017c06d3456eb2beacca))
* **akroasis:** backend integration — proxy, auth, error logging, render loop ([8f875f9](https://github.com/forkwright/harmonia/commit/8f875f966051a44f02c5637e7dbe8f26137593e6))
* **akroasis:** web playback rewrite — streaming HTMLAudioElement, signal path, auth ([baf63ce](https://github.com/forkwright/harmonia/commit/baf63ce197521470c136c684d8f234b48e32e0e6))
* **ci:** bump MSRV check from 1.85 to 1.88 ([#80](https://github.com/forkwright/harmonia/issues/80)) ([d983ddd](https://github.com/forkwright/harmonia/commit/d983ddd9e77c263a2708fed48a1b2d504e4bb593))
* **ci:** disable subject-case rule in commitlint ([bcd5eb8](https://github.com/forkwright/harmonia/commit/bcd5eb829039d331ee83f4d7d4ad6e7299ce3887))
* **ci:** use harmonia-specific binary and features in rust.yml ([#108](https://github.com/forkwright/harmonia/issues/108)) ([e0173cf](https://github.com/forkwright/harmonia/commit/e0173cf785de348b991c5f0ff2fd4ce31ae218a2))
* clippy warnings — unused imports, large_err in tests, collapsible if ([3882e1e](https://github.com/forkwright/harmonia/commit/3882e1ee216b814394d726bac1145eba7227e5fb))
* **docs:** remove stale planned marker from VISION.md link ([342da47](https://github.com/forkwright/harmonia/commit/342da4767103431606a46c93843fbda0a6e86d1f))
* **infra:** CI fixes — cargo fmt flag, advisory ignores, PII redaction ([fd5401e](https://github.com/forkwright/harmonia/commit/fd5401e14ef164d4a67d510df5c68aec1d4b10f8))
* **mouseion:** bug audit — security, OPDS auth, webhook secrets, streaming CSP ([8771085](https://github.com/forkwright/harmonia/commit/8771085a6f601a4c8eba276526ada34315959587))
* **mouseion:** runtime stabilization — DI, SQL types, background services, Swagger ([72fc8c2](https://github.com/forkwright/harmonia/commit/72fc8c2b8ee5ed1eaaa6098895ccc08eddc6d224))
* **mouseion:** security hardening — log injection, path traversal, null safety, resource disposal ([e1e63c8](https://github.com/forkwright/harmonia/commit/e1e63c8d4d72537b4e6058d8e58a2842dbcf825f))
* resolve 4 lint violations via kanon lint --fix ([#140](https://github.com/forkwright/harmonia/issues/140)) ([d9c490f](https://github.com/forkwright/harmonia/commit/d9c490f3c14dd3332a090756063021b5ceae3a27))
* resolve 4 lint violations via kanon lint --fix ([#141](https://github.com/forkwright/harmonia/issues/141)) ([13283cb](https://github.com/forkwright/harmonia/commit/13283cbf765b0f8ca2e0fa9791bac440c2a9f33f))
* resolve 4 lint violations via kanon lint --fix ([#142](https://github.com/forkwright/harmonia/issues/142)) ([0a8b7b1](https://github.com/forkwright/harmonia/commit/0a8b7b14675b00b80064722e4c7d0d48ab0c6844))
* resolve 4 lint violations via kanon lint --fix ([#143](https://github.com/forkwright/harmonia/issues/143)) ([3b9219a](https://github.com/forkwright/harmonia/commit/3b9219a2b90d263ffcef6031f9bde19ac8778798))
* resolve lint violations via kanon lint --fix ([1fc4d5b](https://github.com/forkwright/harmonia/commit/1fc4d5bde98fea5142b855f48e3b1b78e5d8dd52))
* resolve lint violations via kanon lint --fix ([09cb2ab](https://github.com/forkwright/harmonia/commit/09cb2abb30a6ddb992101e72f7fa6d6572d936cf))
* resolve lint violations via kanon lint --fix ([2252080](https://github.com/forkwright/harmonia/commit/2252080e3af42e2b46e3f945a5119266b2488e84))
* resolve lint violations via kanon lint --fix ([102c6c1](https://github.com/forkwright/harmonia/commit/102c6c110ddc5c8e5a21c2cfe0931add856704e8))

## [0.1.4](https://github.com/forkwright/harmonia/compare/v0.1.3...v0.1.4) (2026-04-03)


### Bug Fixes

* resolve lint violations via kanon lint --fix ([09cb2ab](https://github.com/forkwright/harmonia/commit/09cb2abb30a6ddb992101e72f7fa6d6572d936cf))

## [0.1.3](https://github.com/forkwright/harmonia/compare/v0.1.2...v0.1.3) (2026-04-03)


### Features

* **akouo-core:** extract audio engine into workspace crate ([#121](https://github.com/forkwright/harmonia/issues/121)) ([#125](https://github.com/forkwright/harmonia/issues/125)) ([f378e1c](https://github.com/forkwright/harmonia/commit/f378e1c4999eea9ba67b5a4dd29b3afd13a35af7))
* **harmonia-host:** add render subcommand with local DSP and status reporting ([#127](https://github.com/forkwright/harmonia/issues/127)) ([f09f84e](https://github.com/forkwright/harmonia/commit/f09f84e5fd1976209fcb45b0409946911e9962d1))
* **nix:** NixOS renderer module with DAC HAT overlays and aarch64 cross-compilation ([#130](https://github.com/forkwright/harmonia/issues/130)) ([2c45336](https://github.com/forkwright/harmonia/commit/2c45336aa09f57bd401b74afb5b81d4c86c7c0d1))
* **syndesis:** mDNS discovery, pairing protocol, and renderer registry ([#128](https://github.com/forkwright/harmonia/issues/128)) ([31fe086](https://github.com/forkwright/harmonia/commit/31fe08684bb288b80555faa06e3311a27272af9f))
* **syndesis:** multi-room zone grouping with &lt;=5ms clock sync ([#129](https://github.com/forkwright/harmonia/issues/129)) ([f46356b](https://github.com/forkwright/harmonia/commit/f46356b3660c3d808e3071dfa13082fb710e6dd8))
* **syndesis:** QUIC streaming protocol with clock sync and jitter buffer ([#126](https://github.com/forkwright/harmonia/issues/126)) ([ee925c6](https://github.com/forkwright/harmonia/commit/ee925c6f9b9834ba000bf4c4f0efe162556ed8b9))


### Bug Fixes

* add [graph] section to deny.toml for cargo-deny 0.19 compatibility ([f0d0811](https://github.com/forkwright/harmonia/commit/f0d0811d38bb85d14683017c06d3456eb2beacca))
* resolve 4 lint violations via kanon lint --fix ([#140](https://github.com/forkwright/harmonia/issues/140)) ([d9c490f](https://github.com/forkwright/harmonia/commit/d9c490f3c14dd3332a090756063021b5ceae3a27))
* resolve 4 lint violations via kanon lint --fix ([#141](https://github.com/forkwright/harmonia/issues/141)) ([13283cb](https://github.com/forkwright/harmonia/commit/13283cbf765b0f8ca2e0fa9791bac440c2a9f33f))
* resolve 4 lint violations via kanon lint --fix ([#142](https://github.com/forkwright/harmonia/issues/142)) ([0a8b7b1](https://github.com/forkwright/harmonia/commit/0a8b7b14675b00b80064722e4c7d0d48ab0c6844))
* resolve 4 lint violations via kanon lint --fix ([#143](https://github.com/forkwright/harmonia/issues/143)) ([3b9219a](https://github.com/forkwright/harmonia/commit/3b9219a2b90d263ffcef6031f9bde19ac8778798))
* resolve lint violations via kanon lint --fix ([2252080](https://github.com/forkwright/harmonia/commit/2252080e3af42e2b46e3f945a5119266b2488e84))
* resolve lint violations via kanon lint --fix ([102c6c1](https://github.com/forkwright/harmonia/commit/102c6c110ddc5c8e5a21c2cfe0931add856704e8))

## [0.1.2](https://github.com/forkwright/harmonia/compare/v0.1.1...v0.1.2) (2026-03-23)


### Features

* **theatron:** scaffold Dioxus desktop app (phase 0 of [#120](https://github.com/forkwright/harmonia/issues/120)) ([#122](https://github.com/forkwright/harmonia/issues/122)) ([520c8c1](https://github.com/forkwright/harmonia/commit/520c8c1c7fc0553d5380097264ca92d055d6d4dc))

## [0.1.1](https://github.com/forkwright/harmonia/compare/v0.1.0...v0.1.1) (2026-03-18)


### Features

* **aitesis:** request management (P3-05) ([#87](https://github.com/forkwright/harmonia/issues/87)) ([217b523](https://github.com/forkwright/harmonia/commit/217b523303e78e89ec48659084ccd5e94a067419))
* **akroasis-core:** cpal output backend, format negotiation, resampler ([#39](https://github.com/forkwright/harmonia/issues/39)) ([1fc3e61](https://github.com/forkwright/harmonia/commit/1fc3e613f08436597f127a8c84bb97a87b054dc6))
* **akroasis-core:** DSP stages 5–7 — compressor, convolution, volume+dither ([#35](https://github.com/forkwright/harmonia/issues/35)) ([329fb55](https://github.com/forkwright/harmonia/commit/329fb55f2e8b72fbf1a5aae2880a96ae7a8349e6))
* **akroasis-core:** gapless playback scheduler, crossfade, and codec delay trimming ([#40](https://github.com/forkwright/harmonia/issues/40)) ([d69d522](https://github.com/forkwright/harmonia/commit/d69d522ca5c832ef75ca67d9df01ac3d6a5c5b9d))
* **akroasis-core:** scaffold module structure, core types, ring buffer ([#33](https://github.com/forkwright/harmonia/issues/33)) ([9f17877](https://github.com/forkwright/harmonia/commit/9f178777e04f80f80de30f9ac80627193e18f80d))
* **akroasis-core:** Symphonia decode pipeline ([#37](https://github.com/forkwright/harmonia/issues/37)) ([14d2809](https://github.com/forkwright/harmonia/commit/14d2809e63315ab09663d58065e511b7e4d67d8c))
* **akroasis:** add DSP controls UI with crossfeed presets, ReplayGain modes, and output device selector ([#98](https://github.com/forkwright/harmonia/issues/98)) ([7be6105](https://github.com/forkwright/harmonia/commit/7be610568da52509d4b52b05902bfe7002beb04a))
* **akroasis:** Android audio playback — pipeline, auth, queue, media session ([7a4d6f5](https://github.com/forkwright/harmonia/commit/7a4d6f54a5d73a2c9961f219b9b7a77a36809f88))
* **akroasis:** Android audiobook playback, ebook reader, and CI releases ([93aa8dc](https://github.com/forkwright/harmonia/commit/93aa8dcddf2dc67002113f1964aad7e2b66b305c))
* **akroasis:** Android Phase 2 — UI scaffolding, search, audio intelligence ([c493ec3](https://github.com/forkwright/harmonia/commit/c493ec30914d57013f7ee98535c01abe166a3f68))
* **akroasis:** audio DSP, listening DNA, podcast management, and nav redesign ([b19728c](https://github.com/forkwright/harmonia/commit/b19728c9215a7df8dcb23c0b9ea0ce33633ac9fc))
* **akroasis:** audiobook support — library, player, chapters, progress tracking ([154c932](https://github.com/forkwright/harmonia/commit/154c932c9f4a1ffbe8cb1e45d46b9345a04f8b2d))
* **akroasis:** design coherence — warm parchment, serif headings, login defaults ([423ba85](https://github.com/forkwright/harmonia/commit/423ba852adf1e7e97dd79017e621ce01cbf397c0))
* **akroasis:** design system, library browsing, adaptive experience, media-type players ([6e112f8](https://github.com/forkwright/harmonia/commit/6e112f8d4eb998c5cacea29c1d0b903fedfdb423))
* **akroasis:** DSP stages 1–4 — skip silence, parametric EQ, crossfeed, ReplayGain ([#38](https://github.com/forkwright/harmonia/issues/38)) ([41e0425](https://github.com/forkwright/harmonia/commit/41e0425fa23e412886f638cef80ce63e2dee9fa3))
* **akroasis:** engine wiring, PlayQueue, and harmonia CLI (P1-08) ([#41](https://github.com/forkwright/harmonia/issues/41)) ([0a2ab99](https://github.com/forkwright/harmonia/commit/0a2ab99fa03566af47f57e1dbec3fb4de19a1c9a))
* **akroasis:** integration cycle, QA, and CI workflows ([caaacec](https://github.com/forkwright/harmonia/commit/caaacecd0cb3d19b3c5eed1cff0cc3fe46727bec))
* **akroasis:** Opus FFI decoder bridge and WavPack skeleton (P1-03) ([#36](https://github.com/forkwright/harmonia/issues/36)) ([205c3b0](https://github.com/forkwright/harmonia/commit/205c3b03c67f40ee3783c4a8410730822c356379))
* **akroasis:** playback progress tracking and session management ([d772139](https://github.com/forkwright/harmonia/commit/d7721398838d488573ffb175f3f31c11655af01b))
* **akroasis:** sleep timer, bookmarks, lyrics, EQ, Android Auto, artwork zoom ([190527b](https://github.com/forkwright/harmonia/commit/190527b2d2284592d28914cc0c1b23d9be9e5743))
* **akroasis:** test coverage to 80% and voice search integration ([90b222e](https://github.com/forkwright/harmonia/commit/90b222e159bda18629e1a9cd019467eccd03ed47))
* **akroasis:** voice search, A/B comparison, accessibility, settings, and test coverage ([ce8fafa](https://github.com/forkwright/harmonia/commit/ce8fafae845b7414f39ad967cb76948e5185dd77))
* **akroasis:** web auth, discovery, cross-device sync, global search, AutoEQ ([1277f3c](https://github.com/forkwright/harmonia/commit/1277f3c093bbf4cabf4eebae006446bb69d152e9))
* **akroasis:** web bug fixes — auth, API alignment, theme unification ([12758e6](https://github.com/forkwright/harmonia/commit/12758e63347da7b3eca32745bc94b2dffcb7bd6e))
* **akroasis:** web MVP — gapless playback, queue, keyboard shortcuts, PWA ([2cbba2e](https://github.com/forkwright/harmonia/commit/2cbba2ed618a3fd399562696fcbf46db09119dac))
* **akroasis:** web player foundation — mock API server, library browsing, bit-perfect audio ([8563435](https://github.com/forkwright/harmonia/commit/856343509bc32736f4109d4253b9d369843f878a))
* **akroasis:** web UI overhaul — player, library, navigation, design system, playback engine ([dee4c1b](https://github.com/forkwright/harmonia/commit/dee4c1b1816cf6400611e7284824da2b276671c4))
* album art endpoint, sort controls, playlist tracks API ([6a6216a](https://github.com/forkwright/harmonia/commit/6a6216a8b220cdc12465245e372f986a78c48ef6))
* cover art, favorites, playlists, library UI, and artist stats ([c8f6133](https://github.com/forkwright/harmonia/commit/c8f613371f030d36825a5a28a02b8ebbeec2ca42))
* **desktop:** audiobook player with chapters and bookmarks (P3-13) ([#82](https://github.com/forkwright/harmonia/issues/82)) ([26129ee](https://github.com/forkwright/harmonia/commit/26129ee293eb74361ab1d982fa714cb3fce1cc20))
* **desktop:** EQ panel and DSP controls with AutoEQ (P3-12) ([#71](https://github.com/forkwright/harmonia/issues/71)) ([e4b659a](https://github.com/forkwright/harmonia/commit/e4b659a1eb0bbb54f40546723ed8f3c1e74a75c1))
* **desktop:** library browser — album/artist/track views (P3-10) ([#70](https://github.com/forkwright/harmonia/issues/70)) ([487e8ea](https://github.com/forkwright/harmonia/commit/487e8ead30007c3df5074d9228a98607185bfd7a))
* **desktop:** media management UI for all 8 types (P3-15) ([#88](https://github.com/forkwright/harmonia/issues/88)) ([ec96517](https://github.com/forkwright/harmonia/commit/ec96517e5af2dc172e6d579e8752df9b5ed8eddc))
* **desktop:** MPRIS, system tray, and OS integration (P3-16) ([#92](https://github.com/forkwright/harmonia/issues/92)) ([2fac949](https://github.com/forkwright/harmonia/commit/2fac9493113f90d1988281b9ab0da9c0f9ba83a4))
* **desktop:** now playing with playback controls and queue (P3-11) ([#84](https://github.com/forkwright/harmonia/issues/84)) ([93fdf14](https://github.com/forkwright/harmonia/commit/93fdf14247e4d64205f240c95be6350764157a32))
* **desktop:** podcast player with subscriptions and episodes (P3-14) ([#81](https://github.com/forkwright/harmonia/issues/81)) ([917ba1f](https://github.com/forkwright/harmonia/commit/917ba1faece379b84a07fe56ec92633e2ba43fc0))
* **desktop:** Tauri 2 scaffold with React 19 (P3-09) ([1c4b6cb](https://github.com/forkwright/harmonia/commit/1c4b6cbf4c7621d631f698dc51967a882d82277c))
* **epignosis:** metadata enrichment (P2-06) ([#46](https://github.com/forkwright/harmonia/issues/46)) ([516ec30](https://github.com/forkwright/harmonia/commit/516ec307febe7efb834223615dd7a4563b8118b1))
* **ergasia:** download execution and archive extraction (P3-02) ([#69](https://github.com/forkwright/harmonia/issues/69)) ([93582e1](https://github.com/forkwright/harmonia/commit/93582e11e975a37d1485c2aa6c046a25c3ae7412))
* error logging and diagnostics — IndexedDB, server persistence, client log API ([5e25453](https://github.com/forkwright/harmonia/commit/5e25453a695bdab93d3475a3e1a1f56f80ace318))
* **exousia:** authentication and authorization (P2-04) ([#48](https://github.com/forkwright/harmonia/issues/48)) ([358d9a9](https://github.com/forkwright/harmonia/commit/358d9a9c44000d8ee50a85f7bc3fd0ea97bfdda8))
* **harmonia-db:** play history, scrobble tracking, and listening analytics (P2-14) ([#45](https://github.com/forkwright/harmonia/issues/45)) ([88f0e93](https://github.com/forkwright/harmonia/commit/88f0e93864ad7b847e417d88ecddc1cb4c5b9029))
* **harmonia-db:** SQLite database layer with dual-pool WAL (P2-03) ([#44](https://github.com/forkwright/harmonia/issues/44)) ([32a53f9](https://github.com/forkwright/harmonia/commit/32a53f9560133f8a2b2a84479a8fefbdf9effbfc))
* **harmonia-host:** serve mode wiring (P2-12) ([#56](https://github.com/forkwright/harmonia/issues/56)) ([c61d57d](https://github.com/forkwright/harmonia/commit/c61d57d5210032064e66712693c31a357227d29b))
* **harmonia-host:** wire acquisition subsystems into startup/shutdown (P101) ([#100](https://github.com/forkwright/harmonia/issues/100)) ([e82418b](https://github.com/forkwright/harmonia/commit/e82418bd52dd7ad7799a36ad12df11a423879444))
* **horismos:** configuration loading and validation (P2-02) ([#43](https://github.com/forkwright/harmonia/issues/43)) ([ecf6ad5](https://github.com/forkwright/harmonia/commit/ecf6ad532db1b92eff314286706c8ad3be88be51))
* **kritike:** quality profiles and library health (P2-07) ([#47](https://github.com/forkwright/harmonia/issues/47)) ([faa03ed](https://github.com/forkwright/harmonia/commit/faa03ed09a13e3dd5a81168d3ea10660090c4e53))
* **mouseion:** API quality, quality detection, and advanced file import pipeline ([b27037d](https://github.com/forkwright/harmonia/commit/b27037d8f50a86fbd3a7724126fefa95cdb30e14))
* **mouseion:** auth system, import workflows, smart playlists, and build fixes ([c77d909](https://github.com/forkwright/harmonia/commit/c77d909a570bfc4fe5feea0111bdca4fde520ddf))
* **mouseion:** database foundation — MediaItems, SignalR, DI wiring, CI ([33e2bbd](https://github.com/forkwright/harmonia/commit/33e2bbd6b6a6dea320de2241a0c43b9b8dfc6ef6))
* **mouseion:** Docker containerization and production hardening ([f29859a](https://github.com/forkwright/harmonia/commit/f29859a772a03e51dc6f65679e5bdf3e31294209))
* **mouseion:** import wizard, user permissions, acquisition orchestration ([8644b46](https://github.com/forkwright/harmonia/commit/8644b46f12c997481511263bb69e0cf0508e56f8))
* **mouseion:** media scanners, streaming endpoint, and search improvements ([26470f6](https://github.com/forkwright/harmonia/commit/26470f61abff10ab4cba1cd1d86533c8069dd642))
* **mouseion:** music and movie APIs — MusicBrainz, TMDb, file scanning ([2111bac](https://github.com/forkwright/harmonia/commit/2111bac89abaf24e3f6eb5a48695e0ea7c2889ac))
* **mouseion:** news/RSS, manga, comics, health checks, notifications, and validation ([e655d8a](https://github.com/forkwright/harmonia/commit/e655d8a27502a412f4aac8fa9086bdd6ca99520b))
* **mouseion:** OIDC auth, tech debt cleanup, and dependency updates ([b881e11](https://github.com/forkwright/harmonia/commit/b881e111550adf1986f4bb11bb290cb8d5e67684))
* **mouseion:** OpenSubtitles, rate limiting, audiobook chapters, progress tracking, RFC 7807 ([58861c3](https://github.com/forkwright/harmonia/commit/58861c3e7861ec0bd2a30a037920ff47ca89007c))
* **mouseion:** podcast and news feed subscription (P2-11) ([#50](https://github.com/forkwright/harmonia/issues/50)) ([92a97ff](https://github.com/forkwright/harmonia/commit/92a97ff1a4eba397fac357ee1c0a5eb4853d0990))
* **mouseion:** port core infrastructure from Radarr — DI, HTTP, serialization, disk, crypto ([71f99e1](https://github.com/forkwright/harmonia/commit/71f99e1db9152780e13987a8f3cda0d54eb4809c))
* **mouseion:** SIGHUP config reload (P2-15) ([#55](https://github.com/forkwright/harmonia/issues/55)) ([6f6aed1](https://github.com/forkwright/harmonia/commit/6f6aed1cf7c8a15936ed5c81da21e99863979356))
* **mouseion:** TV, podcasts, notifications, download clients, and archive migration ([80acbd9](https://github.com/forkwright/harmonia/commit/80acbd9000e65ce505781c95f0e9676fc278a5b9))
* **mouseion:** TVDB v4 integration, bulk operations, LoggerMessage, and OpenTelemetry ([73bc122](https://github.com/forkwright/harmonia/commit/73bc12229ba365228a8e03189bdfd3452ad104ae))
* **mouseion:** webhook ingestion, OPDS catalog, smart lists, and analytics ([be67f18](https://github.com/forkwright/harmonia/commit/be67f18b8614110d35e656b48dd4fd4d79f856e5))
* NixOS module for declarative deployment (P2-13) ([#57](https://github.com/forkwright/harmonia/issues/57)) ([702237a](https://github.com/forkwright/harmonia/commit/702237af995bb8d902a3e6af74a28db60ead8ca0))
* **paroche:** acquisition API endpoints (P102) ([#101](https://github.com/forkwright/harmonia/issues/101)) ([931e18c](https://github.com/forkwright/harmonia/commit/931e18c5b97628f44fd6507878090cb1dc22224e))
* **paroche:** core HTTP API (P2-08) ([#51](https://github.com/forkwright/harmonia/issues/51)) ([f83361b](https://github.com/forkwright/harmonia/commit/f83361bcd385a8a0a10181386d052ee892d1782a))
* **paroche:** OPDS 2.0 catalog (P2-10) ([#52](https://github.com/forkwright/harmonia/issues/52)) ([f0f89b6](https://github.com/forkwright/harmonia/commit/f0f89b6ef6a12930fa5c919d69af8c097b93e141))
* **paroche:** OpenSubsonic API (P2-09) ([#54](https://github.com/forkwright/harmonia/issues/54)) ([703ee54](https://github.com/forkwright/harmonia/commit/703ee5439fd75deba913b30680f076513b739a2c))
* **prostheke:** subtitle management (P3-07) ([#91](https://github.com/forkwright/harmonia/issues/91)) ([76c1885](https://github.com/forkwright/harmonia/commit/76c1885935e39748ce888ee7cb94e6f9f0cf58d4))
* **syndesmos:** external service integration crate (P3-06) ([#83](https://github.com/forkwright/harmonia/issues/83)) ([459cf97](https://github.com/forkwright/harmonia/commit/459cf971086ee168f80dde8d2deab6934e92bcc8))
* **syntaxis:** queue orchestration and post-processing (P3-03) ([#90](https://github.com/forkwright/harmonia/issues/90)) ([3766e59](https://github.com/forkwright/harmonia/commit/3766e5912da6370a20cebed26ed55fc4bdc44d68))
* **taxis:** library scanner and import pipeline (P2-05) ([#49](https://github.com/forkwright/harmonia/issues/49)) ([2914dc4](https://github.com/forkwright/harmonia/commit/2914dc417e0b0365ef19a5b1c37446272523c0a1))
* workspace scaffold and harmonia-common crate (P2-01) ([#42](https://github.com/forkwright/harmonia/issues/42)) ([6aa8642](https://github.com/forkwright/harmonia/commit/6aa864239e05efa2a5293bf77b2d263ba67c95fa))
* **zetesis:** indexer protocol and search routing (P3-01) ([#59](https://github.com/forkwright/harmonia/issues/59)) ([2b7a4fe](https://github.com/forkwright/harmonia/commit/2b7a4fecd83b7032e3ecdafaf7b5a6b1b7781c38))


### Bug Fixes

* **akroasis:** backend integration — proxy, auth, error logging, render loop ([8f875f9](https://github.com/forkwright/harmonia/commit/8f875f966051a44f02c5637e7dbe8f26137593e6))
* **akroasis:** web playback rewrite — streaming HTMLAudioElement, signal path, auth ([baf63ce](https://github.com/forkwright/harmonia/commit/baf63ce197521470c136c684d8f234b48e32e0e6))
* **ci:** bump MSRV check from 1.85 to 1.88 ([#80](https://github.com/forkwright/harmonia/issues/80)) ([d983ddd](https://github.com/forkwright/harmonia/commit/d983ddd9e77c263a2708fed48a1b2d504e4bb593))
* **ci:** disable subject-case rule in commitlint ([bcd5eb8](https://github.com/forkwright/harmonia/commit/bcd5eb829039d331ee83f4d7d4ad6e7299ce3887))
* **ci:** use harmonia-specific binary and features in rust.yml ([#108](https://github.com/forkwright/harmonia/issues/108)) ([e0173cf](https://github.com/forkwright/harmonia/commit/e0173cf785de348b991c5f0ff2fd4ce31ae218a2))
* clippy warnings — unused imports, large_err in tests, collapsible if ([3882e1e](https://github.com/forkwright/harmonia/commit/3882e1ee216b814394d726bac1145eba7227e5fb))
* **docs:** remove stale planned marker from VISION.md link ([342da47](https://github.com/forkwright/harmonia/commit/342da4767103431606a46c93843fbda0a6e86d1f))
* **infra:** CI fixes — cargo fmt flag, advisory ignores, PII redaction ([fd5401e](https://github.com/forkwright/harmonia/commit/fd5401e14ef164d4a67d510df5c68aec1d4b10f8))
* **mouseion:** bug audit — security, OPDS auth, webhook secrets, streaming CSP ([8771085](https://github.com/forkwright/harmonia/commit/8771085a6f601a4c8eba276526ada34315959587))
* **mouseion:** runtime stabilization — DI, SQL types, background services, Swagger ([72fc8c2](https://github.com/forkwright/harmonia/commit/72fc8c2b8ee5ed1eaaa6098895ccc08eddc6d224))
* **mouseion:** security hardening — log injection, path traversal, null safety, resource disposal ([e1e63c8](https://github.com/forkwright/harmonia/commit/e1e63c8d4d72537b4e6058d8e58a2842dbcf825f))
