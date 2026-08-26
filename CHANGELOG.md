# Changelog

## [0.4.2](https://github.com/forkwright/harmonia/compare/v0.4.1...v0.4.2) (2026-08-26)


### Bug Fixes

* **ci:** name the major the fetch-metadata pin actually is ([#732](https://github.com/forkwright/harmonia/issues/732)) ([c256b3f](https://github.com/forkwright/harmonia/commit/c256b3f6620269760cf72d55e15f8c4dfd12eaf9))

## [0.4.1](https://github.com/forkwright/harmonia/compare/v0.4.0...v0.4.1) (2026-08-24)


### Bug Fixes

* **periskopio:** declare the fleet licence instead of AGPL ([#725](https://github.com/forkwright/harmonia/issues/725)) ([33f2b0b](https://github.com/forkwright/harmonia/commit/33f2b0b55ce7fcb6387f766d0551900914c428d1))

## [0.4.0](https://github.com/forkwright/harmonia/compare/v0.3.0...v0.4.0) (2026-08-19)


### Features

* **theatron-desktop:** periskopio design-token foundation ([#719](https://github.com/forkwright/harmonia/issues/719)) ([564cb2b](https://github.com/forkwright/harmonia/commit/564cb2beae42965dd208ea9a3ddeb99cb43c0884))

## [0.3.0](https://github.com/forkwright/harmonia/compare/v0.2.3...v0.3.0) (2026-08-12)


### Features

* **archon:** cooperative cancellation for the rmcp stdio surface ([#652](https://github.com/forkwright/harmonia/issues/652) PR 3) ([#702](https://github.com/forkwright/harmonia/issues/702)) ([7afdb30](https://github.com/forkwright/harmonia/commit/7afdb30ba075ff21c831e9c1e83fad170e28da8d))
* **archon:** rmcp stdio server replaces the hand-rolled MCP loop ([#701](https://github.com/forkwright/harmonia/issues/701)) ([5f4a5f6](https://github.com/forkwright/harmonia/commit/5f4a5f64b87afc9c2c6eb164caf5534e439f4150)), closes [#652](https://github.com/forkwright/harmonia/issues/652)
* **archon:** start/status/stop lifecycle ops for playback and renderer ([#703](https://github.com/forkwright/harmonia/issues/703)) ([6a89c52](https://github.com/forkwright/harmonia/commit/6a89c52a0aaf61479ce1283df2e8f41fa1038667)), closes [#652](https://github.com/forkwright/harmonia/issues/652)
* **archon:** typed MCP tool parameter DTOs (rmcp migration PR 1) ([#700](https://github.com/forkwright/harmonia/issues/700)) ([858a12f](https://github.com/forkwright/harmonia/commit/858a12fda92af57fc874ca6f1d46f5c7831c5032))
* **eksetasis:** cardigann template blocks, field defaults, and row-scope .Result ([#697](https://github.com/forkwright/harmonia/issues/697)) ([7901ec2](https://github.com/forkwright/harmonia/commit/7901ec242e98b569718d0517c495ee394d302538))
* **syndesmos,horismos:** Tidal OAuth token refresh and scheduled want-list sync ([#695](https://github.com/forkwright/harmonia/issues/695)) ([478f979](https://github.com/forkwright/harmonia/commit/478f9797177af2e16ab866ac8c2633cdf2718bf0))
* **syndesmos,paroche:** Plex collection sync and viewing-stats endpoints ([#698](https://github.com/forkwright/harmonia/issues/698)) ([5d961a2](https://github.com/forkwright/harmonia/commit/5d961a2e0c6876ea3b8fb5f0c94c6528b4eef398))


### Bug Fixes

* **archon,horismos:** constrain MCP socket_path overrides to the owned runtime directory ([#694](https://github.com/forkwright/harmonia/issues/694)) ([cf084e6](https://github.com/forkwright/harmonia/commit/cf084e6991b749c8744888208fc4870eb687dff2))


### Refactoring

* **aggelmata:** rename themelion to aggelmata (D-062) ([#704](https://github.com/forkwright/harmonia/issues/704)) ([ce5d346](https://github.com/forkwright/harmonia/commit/ce5d34627ac02edd70939a8dfc1c3b470432b4e4)), closes [#691](https://github.com/forkwright/harmonia/issues/691)

## [0.2.3](https://github.com/forkwright/harmonia/compare/v0.2.2...v0.2.3) (2026-08-09)


### Bug Fixes

* **llm-corpus:** stop restating volatile tracker state in current_state.toml ([#684](https://github.com/forkwright/harmonia/issues/684)) ([c4bec89](https://github.com/forkwright/harmonia/commit/c4bec8906be99b87349369c17b385dbf8aeb5ac1)), closes [#682](https://github.com/forkwright/harmonia/issues/682)
* **release:** publish the source archive and SBOM the attestations cover ([#683](https://github.com/forkwright/harmonia/issues/683)) ([ec658c4](https://github.com/forkwright/harmonia/commit/ec658c4ea33f6ee3ceb70dc056d2d77f2709080e)), closes [#655](https://github.com/forkwright/harmonia/issues/655)

## [0.2.2](https://github.com/forkwright/harmonia/compare/v0.2.1...v0.2.2) (2026-08-03)


### Bug Fixes

* **deps:** bump event-listener to 5.4.2 for RUSTSEC-2026-0221 ([#679](https://github.com/forkwright/harmonia/issues/679)) ([ce56e10](https://github.com/forkwright/harmonia/commit/ce56e1096e1defda735f9cd02ce36aa52d58a9d9))
* **desktop:** stop the standalone lockfile drifting behind releases ([#675](https://github.com/forkwright/harmonia/issues/675)) ([f07ca78](https://github.com/forkwright/harmonia/commit/f07ca78c856932e37883a7993b455fec65d320fd))
* **epignosis:** hold the book cross-reference test ids in one place ([#676](https://github.com/forkwright/harmonia/issues/676)) ([8f2b9d0](https://github.com/forkwright/harmonia/commit/8f2b9d0c732263ed18d2fce488f36c85e97bfc61))
* **lint:** mark the RFC 6598 range citation as a reviewed reference ([#669](https://github.com/forkwright/harmonia/issues/669)) ([9f56867](https://github.com/forkwright/harmonia/commit/9f5686720eb536df0b1f013ddc45e0003e574a42))
* **syndesis:** give each TLS identity test its own temp directory ([#674](https://github.com/forkwright/harmonia/issues/674)) ([a9ef578](https://github.com/forkwright/harmonia/commit/a9ef57845a2cbc4b425ca5fcf052208df8c3c1a6))

## [0.2.1](https://github.com/forkwright/harmonia/compare/v0.2.0...v0.2.1) (2026-07-28)


### Bug Fixes

* **archon,paroche:** persist want and release rows before enqueue, on both surfaces ([#668](https://github.com/forkwright/harmonia/issues/668)) ([ebc998f](https://github.com/forkwright/harmonia/commit/ebc998f27421b3d8c8ee451c877a7a05fdbac63f))
* **archon:** tag EngineAdapter's dead_code suppression with a held marker ([#633](https://github.com/forkwright/harmonia/issues/633)) ([#662](https://github.com/forkwright/harmonia/issues/662)) ([cafee58](https://github.com/forkwright/harmonia/commit/cafee583cbc1e8e9ec3a9ddfc02c70558bf59fff))

## [0.2.0](https://github.com/forkwright/harmonia/compare/v0.1.14...v0.2.0) (2026-07-23)


### Features

* **horismos,kathodos,archon:** media-type coverage for audiobook/comic/podcast/tv ([#612](https://github.com/forkwright/harmonia/issues/612)) ([#625](https://github.com/forkwright/harmonia/issues/625)) ([23f1ae7](https://github.com/forkwright/harmonia/commit/23f1ae7b58392dbad584aa5fb49cbeffc2bc30d9))
* **zetesis:** andmatch row filter for Cardigann definitions ([#631](https://github.com/forkwright/harmonia/issues/631)) ([dc41e23](https://github.com/forkwright/harmonia/commit/dc41e23c751745e725abaaadc93bf612ce776e27))
* **zetesis:** Cardigann filter tail — diacritics, validate, validfilename, url encode/decode, aliases ([#513](https://github.com/forkwright/harmonia/issues/513)) ([#629](https://github.com/forkwright/harmonia/issues/629)) ([032a72a](https://github.com/forkwright/harmonia/commit/032a72a87c65e33e7a6e2316e4d87e43144e15f1))
* **zetesis:** Cardigann JSON search responses (flat arrays) ([#624](https://github.com/forkwright/harmonia/issues/624)) ([28966a6](https://github.com/forkwright/harmonia/commit/28966a64039fab82833fbbd7c3090861fb499d62))
* **zetesis:** Cardigann nested JSON rows (rows.attribute/multiple + parent switch) ([#513](https://github.com/forkwright/harmonia/issues/513)) ([#627](https://github.com/forkwright/harmonia/issues/627)) ([3e0143f](https://github.com/forkwright/harmonia/commit/3e0143f79886d947ce444ae8524817941dbff007))
* **zetesis:** Cardigann POST search paths ([#621](https://github.com/forkwright/harmonia/issues/621)) ([aa7e286](https://github.com/forkwright/harmonia/commit/aa7e286797435a3754f35f911028d082e7736fe1))


### Bug Fixes

* **deps:** bump serde_with 3.20.0 -&gt; 3.21.0 (GHSA-7gcf-g7xr-8hxj) ([#630](https://github.com/forkwright/harmonia/issues/630)) ([970508c](https://github.com/forkwright/harmonia/commit/970508c5872bee28fe54f4e8d79fe7ecb01b39e0))
* **deps:** repair ulid 3.0 and rubato 4.0 fallout on main ([#660](https://github.com/forkwright/harmonia/issues/660)) ([96027e2](https://github.com/forkwright/harmonia/commit/96027e212ec7bb3114c117bee1a9d90a17cf11bc))
* **epignosis:** preserve book provider identity ([#654](https://github.com/forkwright/harmonia/issues/654)) ([#659](https://github.com/forkwright/harmonia/issues/659)) ([b9b48c2](https://github.com/forkwright/harmonia/commit/b9b48c25b4fc2d9aaaadc4b589d92ee278bb632b))
* **periskopio:** restore standalone compilation ([#620](https://github.com/forkwright/harmonia/issues/620)) ([#657](https://github.com/forkwright/harmonia/issues/657)) ([e46a013](https://github.com/forkwright/harmonia/commit/e46a013cff6c826a7a7e6cd63e58ef676c3a9943))
* **zetesis:** redact all credential query params in logged URLs, not just apikey ([#623](https://github.com/forkwright/harmonia/issues/623)) ([#626](https://github.com/forkwright/harmonia/issues/626)) ([6ec6154](https://github.com/forkwright/harmonia/commit/6ec61549fa1aa9b73d3b858698c1ad5d0e1e4809))


### Documentation

* **manifest:** add docs/MANIFEST.toml ([#647](https://github.com/forkwright/harmonia/issues/647)) ([16b700b](https://github.com/forkwright/harmonia/commit/16b700b3823b31c1c054bc760d621569a6a64cef))
* **planning:** pointer-only vision (kanon-canonical) ([#648](https://github.com/forkwright/harmonia/issues/648)) ([8784a34](https://github.com/forkwright/harmonia/commit/8784a341e5464bbb1ac57fbdfc61c6af21d57145))

## [0.1.14](https://github.com/forkwright/harmonia/compare/v0.1.13...v0.1.14) (2026-07-15)


### Features

* **archon:** MCP acquisition tools (search/enqueue/list/cancel) over a serve-hosted socket ([#617](https://github.com/forkwright/harmonia/issues/617)) ([25347eb](https://github.com/forkwright/harmonia/commit/25347ebd920e386d06afded1c0959d5cc70427ae)), closes [#609](https://github.com/forkwright/harmonia/issues/609)
* **zetesis,apotheke,paroche:** per-indexer Cardigann settings override ([#605](https://github.com/forkwright/harmonia/issues/605)) ([5e121eb](https://github.com/forkwright/harmonia/commit/5e121eb2845f906fc698b4126dbeb5520af436f1))
* **zetesis:** Cardigann form, post, and get login with per-indexer sessions ([#607](https://github.com/forkwright/harmonia/issues/607)) ([361e0e4](https://github.com/forkwright/harmonia/commit/361e0e4a4da18cbc728cd513fe5c9b8e55b72bcc))


### Bug Fixes

* **archon,kathodos:** wire production ImportService (real library import) ([#613](https://github.com/forkwright/harmonia/issues/613)) ([f93fb0b](https://github.com/forkwright/harmonia/commit/f93fb0b9f0c1b41a7ba9a16c6782bf80a9c607d1))
* **epignosis,horismos:** provider credential config surface and fingerprint match thresholds ([#594](https://github.com/forkwright/harmonia/issues/594)) ([4a1dd5e](https://github.com/forkwright/harmonia/commit/4a1dd5ea827f04f3986c79328846df0cdd1bcbaa))
* **ergasia,horismos:** enforce seed thresholds live ([#611](https://github.com/forkwright/harmonia/issues/611)) ([a1c8e7a](https://github.com/forkwright/harmonia/commit/a1c8e7ad9be5fe34b5b79d448f039b78d66a2c54))
* **ergasia,syntaxis,archon:** honest download states and completion wiring ([#606](https://github.com/forkwright/harmonia/issues/606)) ([ed0bcbb](https://github.com/forkwright/harmonia/commit/ed0bcbb8b1ac15d98729abcce6bd66e09bb8f30d))
* **ergasia,syntaxis,prostheke:** wire magnet-resolve timeout, stalled-download detection, and OpenSubtitles login ([#600](https://github.com/forkwright/harmonia/issues/600)) ([d215b22](https://github.com/forkwright/harmonia/commit/d215b22a37d6bea32245221bbdcf1172628df807))
* **komide,paroche,archon:** reactive feed subscription and episode auto-download ([#595](https://github.com/forkwright/harmonia/issues/595)) ([3eab74f](https://github.com/forkwright/harmonia/commit/3eab74f7904b9bf15eaf69b37ba5b75c2b6e945c)), closes [#577](https://github.com/forkwright/harmonia/issues/577)
* **paroche:** OPDS Basic auth, real cover serving, and unthrottled downloads ([#591](https://github.com/forkwright/harmonia/issues/591)) ([53c1553](https://github.com/forkwright/harmonia/commit/53c1553d5dcb41a02022a26f71ac72f8a14a1f20))
* **syndesmos:** honor configured circuit_break_failure_threshold ([#593](https://github.com/forkwright/harmonia/issues/593)) ([fe36867](https://github.com/forkwright/harmonia/commit/fe36867dbcb5298a2758a8385e4a747ce5746041)), closes [#576](https://github.com/forkwright/harmonia/issues/576)
* **zetesis,archon:** wire per-indexer result cap, scheduled caps refresh, and CF cookie reuse ([#599](https://github.com/forkwright/harmonia/issues/599)) ([e3db4fc](https://github.com/forkwright/harmonia/commit/e3db4fc8c2688373a275992546ca59bb97c7ae20))
* **zetesis,paroche,archon:** credentialed enqueue-by-reference with a results cache ([#614](https://github.com/forkwright/harmonia/issues/614)) ([7f4141c](https://github.com/forkwright/harmonia/commit/7f4141ce1cc88aa389ce9451fc0144fabaeba267)), closes [#608](https://github.com/forkwright/harmonia/issues/608)


### Documentation

* align architecture and download docs with the actual config schema ([#603](https://github.com/forkwright/harmonia/issues/603)) ([669b16c](https://github.com/forkwright/harmonia/commit/669b16c591b94880d3954a493a59553e4f3ba71b)), closes [#597](https://github.com/forkwright/harmonia/issues/597)

## [0.1.13](https://github.com/forkwright/harmonia/compare/v0.1.12...v0.1.13) (2026-07-07)


### Features

* **archon,horismos:** live QUIC dual-endpoint drain + HTTP listener rebind on config reload ([#529](https://github.com/forkwright/harmonia/issues/529)) ([#585](https://github.com/forkwright/harmonia/issues/585)) ([e3bc94e](https://github.com/forkwright/harmonia/commit/e3bc94ee70cbe804ac1d5321a94ff207cceb2afd))
* **archon:** rebuild-class config supervisors for scanner and feeds ([#529](https://github.com/forkwright/harmonia/issues/529)) ([#586](https://github.com/forkwright/harmonia/issues/586)) ([aacef69](https://github.com/forkwright/harmonia/commit/aacef692516ca9130e2d9bcf34650bbaabe8a0cc))
* **epignosis,prostheke,syndesmos,kritike,aitesis:** live integration-service config ([#529](https://github.com/forkwright/harmonia/issues/529)) ([#588](https://github.com/forkwright/harmonia/issues/588)) ([57e2eb2](https://github.com/forkwright/harmonia/commit/57e2eb28d7a83e04c287026bfe7ee89ab9147541))
* **exousia:** live JWT secret/TTL via config reload with immediate rotation ([#529](https://github.com/forkwright/harmonia/issues/529)) ([#583](https://github.com/forkwright/harmonia/issues/583)) ([77f75f7](https://github.com/forkwright/harmonia/commit/77f75f717b6d766832fd80175b10d2fba50126d0))
* **horismos,archon,paroche:** reactive config foundation — SectionWatcher, live paroche reads, honest SIGHUP logging ([#529](https://github.com/forkwright/harmonia/issues/529)) ([#582](https://github.com/forkwright/harmonia/issues/582)) ([03a00af](https://github.com/forkwright/harmonia/commit/03a00aff55238f7c398509b11e2e63572ab7240a))
* **themelion,archon:** live renderer QUIC api-key/timeout/admission via LiveGate ([#529](https://github.com/forkwright/harmonia/issues/529)) ([#584](https://github.com/forkwright/harmonia/issues/584)) ([7db4ef5](https://github.com/forkwright/harmonia/commit/7db4ef5929d0cda97d75b3d6aea1944d6ad208b5))
* **zetesis,syntaxis,ergasia,horismos:** live acquisition config + seed-threshold restart reclass ([#529](https://github.com/forkwright/harmonia/issues/529)) ([#587](https://github.com/forkwright/harmonia/issues/587)) ([a3f72f0](https://github.com/forkwright/harmonia/commit/a3f72f09a8ce50ad14ebe48d9335a1167273eee3))


### Bug Fixes

* **akouo-core:** stop collapsing &gt;2-channel Opus to stereo ([#544](https://github.com/forkwright/harmonia/issues/544)) ([#571](https://github.com/forkwright/harmonia/issues/571)) ([809d301](https://github.com/forkwright/harmonia/commit/809d301a0825603f4afdc6d216ba5538a34012e5))
* **archon:** guard migrate slices, cap QUIC admission, reset backoff, fix underrun warning ([#566](https://github.com/forkwright/harmonia/issues/566)) ([94d4fd9](https://github.com/forkwright/harmonia/commit/94d4fd94c10af9aad860369a79f4fd7d6430a6d4)), closes [#545](https://github.com/forkwright/harmonia/issues/545) [#546](https://github.com/forkwright/harmonia/issues/546) [#547](https://github.com/forkwright/harmonia/issues/547) [#553](https://github.com/forkwright/harmonia/issues/553)
* **config:** validate scan_concurrency, wire db-pool/kritike/opensubtitles config ([#573](https://github.com/forkwright/harmonia/issues/573)) ([86f20b6](https://github.com/forkwright/harmonia/commit/86f20b69f3dd9056f313ad32b7eaff935f6da5e8))
* **deps:** bump audiopus to build vendored Opus with modern CMake ([#569](https://github.com/forkwright/harmonia/issues/569)) ([b7daf92](https://github.com/forkwright/harmonia/commit/b7daf92bba76113da44cc5c448eb812bdac65557)), closes [#565](https://github.com/forkwright/harmonia/issues/565)
* **ergasia,exousia:** ref-count shared torrent_ids and constant-time login miss-path ([#536](https://github.com/forkwright/harmonia/issues/536), [#540](https://github.com/forkwright/harmonia/issues/540)) ([#572](https://github.com/forkwright/harmonia/issues/572)) ([b899fee](https://github.com/forkwright/harmonia/commit/b899fee460f91cc6476f80dc800bc6346241a70f))
* **metadata:** cap OpenSubtitles bodies, evict metadata cache, bound komide fetch ([#568](https://github.com/forkwright/harmonia/issues/568)) ([fcf46e3](https://github.com/forkwright/harmonia/commit/fcf46e3b568071e828040cfb71d0afcd64d68f4d))
* **paroche:** register OPDS download/cover/content routes ([#530](https://github.com/forkwright/harmonia/issues/530)) ([#574](https://github.com/forkwright/harmonia/issues/574)) ([77d3e91](https://github.com/forkwright/harmonia/commit/77d3e918953d431c03be2e3811fe1f49cf2b1768))
* **zetesis:** correct regexp-filter group semantics and rate-limit embargo accrual ([#531](https://github.com/forkwright/harmonia/issues/531), [#533](https://github.com/forkwright/harmonia/issues/533)) ([#570](https://github.com/forkwright/harmonia/issues/570)) ([d9a250c](https://github.com/forkwright/harmonia/commit/d9a250c4d3c424869c1ee7e3b4ef5b992d6ffc3b))

## [0.1.12](https://github.com/forkwright/harmonia/compare/v0.1.11...v0.1.12) (2026-07-06)


### Features

* **horismos:** reactive config core — leaf diff, held-back merge, Section, ReloadOutcome ([#529](https://github.com/forkwright/harmonia/issues/529) step 1) ([#563](https://github.com/forkwright/harmonia/issues/563)) ([6f72e5f](https://github.com/forkwright/harmonia/commit/6f72e5f4e2733f6998d1fb5b0cf6aeaad60961f9))
* **zetesis:** wire Cardigann definition-driven indexer client ([#512](https://github.com/forkwright/harmonia/issues/512)) ([b53885a](https://github.com/forkwright/harmonia/commit/b53885ad690c3e9207cde0df3fcc1a7626c8fa34))


### Bug Fixes

* **acquisition:** low-severity audit batch findings (zetesis/ergasia/syntaxis/aitesis/syndesmos) ([#503](https://github.com/forkwright/harmonia/issues/503)) ([2a112c1](https://github.com/forkwright/harmonia/commit/2a112c1608b396434ad6017d34c0f7ebe1533725)), closes [#447](https://github.com/forkwright/harmonia/issues/447) [#381](https://github.com/forkwright/harmonia/issues/381) [#461](https://github.com/forkwright/harmonia/issues/461) [#446](https://github.com/forkwright/harmonia/issues/446) [#435](https://github.com/forkwright/harmonia/issues/435) [#445](https://github.com/forkwright/harmonia/issues/445)
* **aitesis:** atomic approval workflow and request authorization ([#487](https://github.com/forkwright/harmonia/issues/487)) ([a27fd12](https://github.com/forkwright/harmonia/commit/a27fd12d498c181e47573657131a1d840445f43a)), closes [#395](https://github.com/forkwright/harmonia/issues/395) [#396](https://github.com/forkwright/harmonia/issues/396) [#397](https://github.com/forkwright/harmonia/issues/397)
* **aitesis:** compare-and-swap request status to close approval lost-update race ([#560](https://github.com/forkwright/harmonia/issues/560)) ([77a3a5e](https://github.com/forkwright/harmonia/commit/77a3a5ea5515156d23f1b42d7288ffc433f286f7))
* **akouo:** playback-engine correctness (real seek, decode-failure signaling) ([#485](https://github.com/forkwright/harmonia/issues/485)) ([8c027af](https://github.com/forkwright/harmonia/commit/8c027af3508b115d786bbe6d84948ea39c701bd0)), closes [#386](https://github.com/forkwright/harmonia/issues/386) [#387](https://github.com/forkwright/harmonia/issues/387) [#398](https://github.com/forkwright/harmonia/issues/398) [#399](https://github.com/forkwright/harmonia/issues/399) [#400](https://github.com/forkwright/harmonia/issues/400) [#401](https://github.com/forkwright/harmonia/issues/401) [#402](https://github.com/forkwright/harmonia/issues/402) [#403](https://github.com/forkwright/harmonia/issues/403) [#404](https://github.com/forkwright/harmonia/issues/404)
* **akouo:** propagate real underruns, stop audio on pause, honor buffer_size ([#562](https://github.com/forkwright/harmonia/issues/562)) ([ddd5097](https://github.com/forkwright/harmonia/commit/ddd509763debea89903b97c2f4d41a583b8f4c93))
* **apotheke:** guard single-row writes against zero-row match ([#507](https://github.com/forkwright/harmonia/issues/507)) ([29a97cc](https://github.com/forkwright/harmonia/commit/29a97cce8cca38e81c0053773b2dd1782d3933fd)), closes [#492](https://github.com/forkwright/harmonia/issues/492)
* **apotheke:** guard want.rs and play_history single-row writes against zero-row match ([#561](https://github.com/forkwright/harmonia/issues/561)) ([3277278](https://github.com/forkwright/harmonia/commit/32772784302d66a3ded24cad75a834f9defdea6a)), closes [#538](https://github.com/forkwright/harmonia/issues/538) [#552](https://github.com/forkwright/harmonia/issues/552)
* **apotheke:** period-scoped stats, checked writes, transactional streaks ([#496](https://github.com/forkwright/harmonia/issues/496)) ([9673863](https://github.com/forkwright/harmonia/commit/96738635818182f0c55974c2e1922dcb6422f571)), closes [#405](https://github.com/forkwright/harmonia/issues/405) [#406](https://github.com/forkwright/harmonia/issues/406) [#407](https://github.com/forkwright/harmonia/issues/407) [#450](https://github.com/forkwright/harmonia/issues/450) [#451](https://github.com/forkwright/harmonia/issues/451)
* **archon:** render reliability and wire metadata/queue/curation traits ([#497](https://github.com/forkwright/harmonia/issues/497)) ([9a57b03](https://github.com/forkwright/harmonia/commit/9a57b0366d7dcda369c15db864e161272cdbe807)), closes [#388](https://github.com/forkwright/harmonia/issues/388) [#408](https://github.com/forkwright/harmonia/issues/408) [#409](https://github.com/forkwright/harmonia/issues/409) [#410](https://github.com/forkwright/harmonia/issues/410) [#411](https://github.com/forkwright/harmonia/issues/411) [#412](https://github.com/forkwright/harmonia/issues/412) [#468](https://github.com/forkwright/harmonia/issues/468) [#469](https://github.com/forkwright/harmonia/issues/469) [#470](https://github.com/forkwright/harmonia/issues/470)
* **archon:** secure renderer TLS/QUIC transport (cert pinning, peer auth) ([#486](https://github.com/forkwright/harmonia/issues/486)) ([cf1fb43](https://github.com/forkwright/harmonia/commit/cf1fb43b4c26c358152152e1b58fcf2579a0b9e2)), closes [#389](https://github.com/forkwright/harmonia/issues/389) [#413](https://github.com/forkwright/harmonia/issues/413) [#414](https://github.com/forkwright/harmonia/issues/414) [#415](https://github.com/forkwright/harmonia/issues/415)
* **core:** low-severity audit batch findings (exousia/horismos/apotheke/kritike) ([#502](https://github.com/forkwright/harmonia/issues/502)) ([790617a](https://github.com/forkwright/harmonia/commit/790617aedb09a3e5b0c56853295105899456114d))
* **epignosis:** metadata provider correctness (TVDB cache, TMDB cross-ref, AcoustID) ([#494](https://github.com/forkwright/harmonia/issues/494)) ([58efa38](https://github.com/forkwright/harmonia/commit/58efa38aec3d671b455a42eedd5a1cc014f61249)), closes [#416](https://github.com/forkwright/harmonia/issues/416) [#417](https://github.com/forkwright/harmonia/issues/417) [#471](https://github.com/forkwright/harmonia/issues/471) [#458](https://github.com/forkwright/harmonia/issues/458)
* **epignosis:** reject non-success HTTP responses and deflake rate-limit test ([#506](https://github.com/forkwright/harmonia/issues/506)) ([995208b](https://github.com/forkwright/harmonia/commit/995208b1bebc539a5f136b93de50192643cb41d4)), closes [#498](https://github.com/forkwright/harmonia/issues/498) [#482](https://github.com/forkwright/harmonia/issues/482)
* **ergasia:** download filesystem safety (reconciliation, disk guard, zip-slip) ([#491](https://github.com/forkwright/harmonia/issues/491)) ([f8ec16e](https://github.com/forkwright/harmonia/commit/f8ec16eaae1ad38b444a43163c92ac80830a5193)), closes [#360](https://github.com/forkwright/harmonia/issues/360) [#365](https://github.com/forkwright/harmonia/issues/365) [#366](https://github.com/forkwright/harmonia/issues/366) [#367](https://github.com/forkwright/harmonia/issues/367) [#452](https://github.com/forkwright/harmonia/issues/452) [#453](https://github.com/forkwright/harmonia/issues/453) [#454](https://github.com/forkwright/harmonia/issues/454)
* **ergasia:** validate archive entries and enforce extraction byte caps ([#555](https://github.com/forkwright/harmonia/issues/555)) ([3353751](https://github.com/forkwright/harmonia/commit/33537511e412ae90faecaa7f4b2d5fefa0579ef5))
* **exousia:** harden auth paths (constant-time keys, DB-authoritative bearer, refresh TOCTOU) ([#476](https://github.com/forkwright/harmonia/issues/476)) ([fae78ab](https://github.com/forkwright/harmonia/commit/fae78abb8d93bb3e6e0f9e2f680610edd38d8015)), closes [#473](https://github.com/forkwright/harmonia/issues/473) [#418](https://github.com/forkwright/harmonia/issues/418) [#419](https://github.com/forkwright/harmonia/issues/419) [#420](https://github.com/forkwright/harmonia/issues/420)
* **komide:** bound feed/episode fetch, fix backoff overflow, add coverage ([#495](https://github.com/forkwright/harmonia/issues/495)) ([bb59ff4](https://github.com/forkwright/harmonia/commit/bb59ff40e44e64fe2622c1f8b3ab0a985f64b1bd)), closes [#368](https://github.com/forkwright/harmonia/issues/368) [#369](https://github.com/forkwright/harmonia/issues/369) [#370](https://github.com/forkwright/harmonia/issues/370) [#371](https://github.com/forkwright/harmonia/issues/371) [#455](https://github.com/forkwright/harmonia/issues/455) [#456](https://github.com/forkwright/harmonia/issues/456)
* **media:** low-severity audit batch findings (komide/epignosis/prostheke) ([#504](https://github.com/forkwright/harmonia/issues/504)) ([9405fb6](https://github.com/forkwright/harmonia/commit/9405fb62fe45ff5954f286df71733056510d82af)), closes [#382](https://github.com/forkwright/harmonia/issues/382) [#464](https://github.com/forkwright/harmonia/issues/464) [#440](https://github.com/forkwright/harmonia/issues/440) [#444](https://github.com/forkwright/harmonia/issues/444) [#466](https://github.com/forkwright/harmonia/issues/466)
* **paroche:** accept repeated Subsonic playlist song params ([#509](https://github.com/forkwright/harmonia/issues/509)) ([86e421e](https://github.com/forkwright/harmonia/commit/86e421e37662a6e5f08f221d88ee1d9b11b3baa7)), closes [#477](https://github.com/forkwright/harmonia/issues/477)
* **paroche:** authenticate endpoints, SSRF guard, real pagination, transactional writes ([#483](https://github.com/forkwright/harmonia/issues/483)) ([7faa6ca](https://github.com/forkwright/harmonia/commit/7faa6ca4181ea1fa5f72c754f24936130f021af1)), closes [#361](https://github.com/forkwright/harmonia/issues/361) [#372](https://github.com/forkwright/harmonia/issues/372) [#373](https://github.com/forkwright/harmonia/issues/373) [#374](https://github.com/forkwright/harmonia/issues/374) [#375](https://github.com/forkwright/harmonia/issues/375) [#457](https://github.com/forkwright/harmonia/issues/457)
* **paroche:** close IPv4-compatible-IPv6 SSRF bypass and resolve magnet trackers ([#511](https://github.com/forkwright/harmonia/issues/511)) ([9ea2ec2](https://github.com/forkwright/harmonia/commit/9ea2ec2e602d4d1c59a152bba23b8b0308aeb51b)), closes [#479](https://github.com/forkwright/harmonia/issues/479)
* **paroche:** report real list totals instead of page-slice length ([#508](https://github.com/forkwright/harmonia/issues/508)) ([2056a1c](https://github.com/forkwright/harmonia/commit/2056a1ce0ba97c1bac0216c7c9d3b0fdaede621d)), closes [#478](https://github.com/forkwright/harmonia/issues/478)
* **paroche:** route enqueue_download through the live download queue ([#510](https://github.com/forkwright/harmonia/issues/510)) ([5356330](https://github.com/forkwright/harmonia/commit/5356330cd0a01c23983ceea91794542268ecec47)), closes [#499](https://github.com/forkwright/harmonia/issues/499)
* **paroche:** scope get_request to caller and redact indexer creds in download_url ([#558](https://github.com/forkwright/harmonia/issues/558)) ([eb906b2](https://github.com/forkwright/harmonia/commit/eb906b2d09fffb3658345029fe937e042e8193b1))
* **serving:** low-severity audit batch findings (paroche/syndesis/archon/akouo) ([#505](https://github.com/forkwright/harmonia/issues/505)) ([8b6c86f](https://github.com/forkwright/harmonia/commit/8b6c86f85194b78dc5a26929406c52e574ccfa4c)), closes [#383](https://github.com/forkwright/harmonia/issues/383) [#465](https://github.com/forkwright/harmonia/issues/465) [#384](https://github.com/forkwright/harmonia/issues/384) [#439](https://github.com/forkwright/harmonia/issues/439) [#437](https://github.com/forkwright/harmonia/issues/437) [#436](https://github.com/forkwright/harmonia/issues/436)
* **syndesis:** secure renderer client (pinned TLS, reassembly, bounded buffers) ([#484](https://github.com/forkwright/harmonia/issues/484)) ([e71cd8d](https://github.com/forkwright/harmonia/commit/e71cd8d6384eb97a77b4903658a2e52395e644ba)), closes [#362](https://github.com/forkwright/harmonia/issues/362) [#363](https://github.com/forkwright/harmonia/issues/363) [#364](https://github.com/forkwright/harmonia/issues/364) [#376](https://github.com/forkwright/harmonia/issues/376) [#377](https://github.com/forkwright/harmonia/issues/377) [#378](https://github.com/forkwright/harmonia/issues/378) [#379](https://github.com/forkwright/harmonia/issues/379) [#380](https://github.com/forkwright/harmonia/issues/380)
* **syndesmos:** correct Last.fm and Tidal integrations ([#481](https://github.com/forkwright/harmonia/issues/481)) ([da2afc3](https://github.com/forkwright/harmonia/commit/da2afc3c612efc6fcf259e246aab08357c5b0b7e)), closes [#390](https://github.com/forkwright/harmonia/issues/390) [#391](https://github.com/forkwright/harmonia/issues/391) [#392](https://github.com/forkwright/harmonia/issues/392) [#421](https://github.com/forkwright/harmonia/issues/421) [#422](https://github.com/forkwright/harmonia/issues/422) [#423](https://github.com/forkwright/harmonia/issues/423)
* **syntaxis:** async mock extract to match ergasia DownloadEngine trait ([#493](https://github.com/forkwright/harmonia/issues/493)) ([593758e](https://github.com/forkwright/harmonia/commit/593758e97c28b6960ff28edbd7d281c155365e8f))
* **syntaxis:** correct download dispatch (slot leaks, retry wakeup, protocol guard) ([#489](https://github.com/forkwright/harmonia/issues/489)) ([e11fd19](https://github.com/forkwright/harmonia/commit/e11fd19f1811ac45e71fd47e67b6d99d7b64efbe)), closes [#393](https://github.com/forkwright/harmonia/issues/393) [#394](https://github.com/forkwright/harmonia/issues/394) [#424](https://github.com/forkwright/harmonia/issues/424) [#425](https://github.com/forkwright/harmonia/issues/425) [#426](https://github.com/forkwright/harmonia/issues/426) [#427](https://github.com/forkwright/harmonia/issues/427) [#428](https://github.com/forkwright/harmonia/issues/428)
* **syntaxis:** dispatch recovered items and close retry/cancel races ([#559](https://github.com/forkwright/harmonia/issues/559)) ([e017616](https://github.com/forkwright/harmonia/commit/e01761613bc338ce504359d2fbebcb449704d05a))
* **zetesis:** close indexer-fetch SSRF, injection, XML-DoS, and key-leak defects ([#556](https://github.com/forkwright/harmonia/issues/556)) ([f3bafb2](https://github.com/forkwright/harmonia/commit/f3bafb22f03aaa6388b93d78a5ff6b4f83bddb54))
* **zetesis:** harden indexer search (SSRF guard, key redaction, body cap, tx) ([#488](https://github.com/forkwright/harmonia/issues/488)) ([9b9ae1b](https://github.com/forkwright/harmonia/commit/9b9ae1b55cf817745a0cd20c6a8e550869fdd7b3)), closes [#429](https://github.com/forkwright/harmonia/issues/429) [#430](https://github.com/forkwright/harmonia/issues/430) [#431](https://github.com/forkwright/harmonia/issues/431) [#432](https://github.com/forkwright/harmonia/issues/432) [#433](https://github.com/forkwright/harmonia/issues/433) [#434](https://github.com/forkwright/harmonia/issues/434) [#459](https://github.com/forkwright/harmonia/issues/459) [#460](https://github.com/forkwright/harmonia/issues/460) [#467](https://github.com/forkwright/harmonia/issues/467)

## [0.1.11](https://github.com/forkwright/harmonia/compare/v0.1.10...v0.1.11) (2026-05-29)


### Features

* **akouo-android:** scaffold UniFFI playback bridge ([7397f09](https://github.com/forkwright/harmonia/commit/7397f0922381fec5b5fe28e52655a213e39591e5))
* **archon:** add MCP command surface ([560bdf7](https://github.com/forkwright/harmonia/commit/560bdf7e7d67472e7c491d7640c8d0970237fe49)), closes [#248](https://github.com/forkwright/harmonia/issues/248)
* **theatron:** add desktop design tokens ([b61ace6](https://github.com/forkwright/harmonia/commit/b61ace6c7168970a643fa65f1a5c7e9c37816f12))


### Bug Fixes

* **akouo-core:** annotate gapless false positives + suppress Arc&lt;Mutex&gt; + process::Command ([#311](https://github.com/forkwright/harmonia/issues/311)) ([a8f90dc](https://github.com/forkwright/harmonia/commit/a8f90dc8fbe87b2f99ff403ac054144c8991057a))
* **akouo-core:** force PipeWire output rate ([#268](https://github.com/forkwright/harmonia/issues/268)) ([f28041f](https://github.com/forkwright/harmonia/commit/f28041f583a16735cb93274345900e496554a8af))
* **akouo-core:** wrap open_decoder blocking file I/O in spawn_blocking ([#325](https://github.com/forkwright/harmonia/issues/325)) ([734b8b9](https://github.com/forkwright/harmonia/commit/734b8b9161c091e86c4155be7011622580883f57))
* **akouo:** return unsupported error for WavPack decoder ([#255](https://github.com/forkwright/harmonia/issues/255)) ([986e0e8](https://github.com/forkwright/harmonia/commit/986e0e8a832648baa537626e659e7786f27bb06e))
* **archon:** annotate false-positive unwrap_or_default on bounded conversions and Option chains ([#320](https://github.com/forkwright/harmonia/issues/320)) ([e6b0430](https://github.com/forkwright/harmonia/commit/e6b04303ad51e1f570ae565eda9b5d6e4bbdc703))
* **archon:** fail unwired import stub ([#257](https://github.com/forkwright/harmonia/issues/257)) ([8ed3b52](https://github.com/forkwright/harmonia/commit/8ed3b529e37d0d9b83de5f96e4a0edb0e4d27cca))
* **archon:** run db migrate command ([#256](https://github.com/forkwright/harmonia/issues/256)) ([7f2e14d](https://github.com/forkwright/harmonia/commit/7f2e14dacb91941763023f9f7c255101e5c23c4f))
* **ci:** add SLSA attestation, fix high kanon violations ([#297](https://github.com/forkwright/harmonia/issues/297)) ([c1c9819](https://github.com/forkwright/harmonia/commit/c1c981955be35119e15ea7574696b90c33e34d4e))
* **docs:** clarify desktop package boundary ([#261](https://github.com/forkwright/harmonia/issues/261)) ([6df6edb](https://github.com/forkwright/harmonia/commit/6df6edbf91d49a271433bcd3e5af67fc55550ef4))
* **docs:** remove stale episkope subsystem references ([3b22aa2](https://github.com/forkwright/harmonia/commit/3b22aa2f01b1fa526adb1c51c50540b4df6168f7))
* **epignosis:** rename EpignosisService → ProviderBackedResolver; fix Option/Result false positives ([#314](https://github.com/forkwright/harmonia/issues/314)) ([7c277ff](https://github.com/forkwright/harmonia/commit/7c277ffb825d45f014f3c1eb34850e23605f9125))
* **epignosis:** suppress false-positive no-result-unwrap-or-default on Option chain ([#323](https://github.com/forkwright/harmonia/issues/323)) ([867a863](https://github.com/forkwright/harmonia/commit/867a863ab64dedfb67d6286bc3cc6a949b2057d8))
* **ergasia:** rename ErgasiaSession + suppress no-direct-process-command ([#310](https://github.com/forkwright/harmonia/issues/310)) ([20d35fe](https://github.com/forkwright/harmonia/commit/20d35fe34d143d19f0a0515168252e08801fa0b1))
* **exousia:** annotate SystemTime false positives (no-result-unwrap-or-default) ([#308](https://github.com/forkwright/harmonia/issues/308)) ([22d4d07](https://github.com/forkwright/harmonia/commit/22d4d07a71d954125961646d0a087f00b08e1e38))
* **horismos:** rename ZetesisConfig + annotate serde false positives ([#309](https://github.com/forkwright/harmonia/issues/309)) ([170900a](https://github.com/forkwright/harmonia/commit/170900a2748df689a2b4da89ea332647800d8971))
* **kathodos:** fix indexing-slicing violations with safe boundary annotations ([#316](https://github.com/forkwright/harmonia/issues/316)) ([c778099](https://github.com/forkwright/harmonia/commit/c7780998dc5abeb516be63bfb636e41b57ff8b2e))
* **kathodos:** suppress false-positive no-blocking-io in spawn_blocking closure ([#321](https://github.com/forkwright/harmonia/issues/321)) ([94e86cb](https://github.com/forkwright/harmonia/commit/94e86cb2492e7fa5c0d7779d7f9b1aef962fb01c))
* **komide:** rename KomideService → FeedSchedulerService; fix i64/u64 conversion false positives ([#317](https://github.com/forkwright/harmonia/issues/317)) ([3025302](https://github.com/forkwright/harmonia/commit/30253029230994be37be6e6299778cc4c0cff2bc))
* **kritike:** proper Result handling (no-result-unwrap-or-default) ([#307](https://github.com/forkwright/harmonia/issues/307)) ([3f5b1bb](https://github.com/forkwright/harmonia/commit/3f5b1bb26fcd94c8c80429998797b0aadef42629))
* **kritike:** remove unwired import registration stub ([#259](https://github.com/forkwright/harmonia/issues/259)) ([f4872a6](https://github.com/forkwright/harmonia/commit/f4872a6db6d1761775c027f07dfba514b6498fee))
* **lint:** spotless-pass — clear all targeted medium/high violations (486 → 335) ([#303](https://github.com/forkwright/harmonia/issues/303)) ([0f3bab9](https://github.com/forkwright/harmonia/commit/0f3bab9c5c2e24271088dca7a462e073bb6aa6ed))
* **nix:** manage ebook conversion tools ([#258](https://github.com/forkwright/harmonia/issues/258)) ([0393170](https://github.com/forkwright/harmonia/commit/03931705ebc9985a3041b7c49c23e0129d4213a2))
* **paroche:** proper Result handling (no-result-unwrap-or-default) ([#304](https://github.com/forkwright/harmonia/issues/304)) ([dfad2a7](https://github.com/forkwright/harmonia/commit/dfad2a7ca5f5c9967311dcfe71ca42241adf6327))
* **paroche:** route requests through aitesis ([50c008e](https://github.com/forkwright/harmonia/commit/50c008eace85ea4f8e6a0051b27eff891201063c)), closes [#247](https://github.com/forkwright/harmonia/issues/247)
* **paroche:** suppress false-positive indexing-slicing in embedded JS string ([#324](https://github.com/forkwright/harmonia/issues/324)) ([733dadd](https://github.com/forkwright/harmonia/commit/733daddb4629b83d44600fdd94387a44402235c2))
* **prostheke:** rename ProsthekeService → SubtitleManager; fix opensubtitles Result handling ([#313](https://github.com/forkwright/harmonia/issues/313)) ([c444f0a](https://github.com/forkwright/harmonia/commit/c444f0aa56e1dcffd327a872680fac4514fd9dc2))
* **search:** wire live zetesis adapter ([#265](https://github.com/forkwright/harmonia/issues/265)) ([6810701](https://github.com/forkwright/harmonia/commit/681070166ac518c4526025a00eba743aa4638a2b))
* **serve:** wire live marker services ([#267](https://github.com/forkwright/harmonia/issues/267)) ([f1c754d](https://github.com/forkwright/harmonia/commit/f1c754d5339c21dc08a19f6ddfbd6c1eebc62ad9)), closes [#241](https://github.com/forkwright/harmonia/issues/241)
* **subtitles:** wire live prostheke adapter ([#266](https://github.com/forkwright/harmonia/issues/266)) ([aa7c30d](https://github.com/forkwright/harmonia/commit/aa7c30d0e14a1e19dca374c202289d552c234574)), closes [#241](https://github.com/forkwright/harmonia/issues/241)
* **syndesis:** fix timestamp conversion false positives; suppress indexing-slicing in protocol buffers ([#318](https://github.com/forkwright/harmonia/issues/318)) ([2992c63](https://github.com/forkwright/harmonia/commit/2992c6370160b76994ec77cc24b8f7adb745fecc))
* **syndesmos:** rename SyndesmosService → ScrobbleClient; fix reqwest/Option false positives ([#315](https://github.com/forkwright/harmonia/issues/315)) ([8d0d4de](https://github.com/forkwright/harmonia/commit/8d0d4dec95353087f039918b2e306c81452469a7))
* **syntaxis:** rename SyntaxisService + suppress Arc&lt;Mutex&gt; + annotate priority ([#312](https://github.com/forkwright/harmonia/issues/312)) ([469a446](https://github.com/forkwright/harmonia/commit/469a446d74db6af62354bbe9853713a64155fbd5))
* **theatron:** annotate reqwest build fallback; suppress no-result-unwrap-or-default ([#322](https://github.com/forkwright/harmonia/issues/322)) ([1b90ac0](https://github.com/forkwright/harmonia/commit/1b90ac0da9419abab3a3338cfed3828382b0c01c))
* **workspace:** restore rust 1.85 compatibility ([d28774d](https://github.com/forkwright/harmonia/commit/d28774d40935b4fd78bf196e06fa05bdaa0f077d))
* **workspace:** satisfy rust 1.95 clippy ([fed8504](https://github.com/forkwright/harmonia/commit/fed850485fc960c3bffc690c34d09beeb18b42ee))
* **zetesis:** rename ZetesisService → SearchIndexerService, ZetesisError → SearchIndexerError; fix reqwest/Option false positives ([#319](https://github.com/forkwright/harmonia/issues/319)) ([1c4299f](https://github.com/forkwright/harmonia/commit/1c4299f9292d871bf95dd9245b117f1e32ed351c))

## [0.1.10](https://github.com/forkwright/harmonia/compare/v0.1.9...v0.1.10) (2026-05-22)


### Features

* **_llm:** add T0 corpus per [#667](https://github.com/forkwright/harmonia/issues/667) / [#673](https://github.com/forkwright/harmonia/issues/673) fleet rollout ([#18](https://github.com/forkwright/harmonia/issues/18)) ([53892ea](https://github.com/forkwright/harmonia/commit/53892ea4f0b06251102dcaf9a9c8473c73d241a6))


### Bug Fixes

* **archon,akouo-core:** green kanon-lint stage ([#12](https://github.com/forkwright/harmonia/issues/12)) ([8db2590](https://github.com/forkwright/harmonia/commit/8db259084164a98a6f872fc865a1b80eeaf9d9b6))
* **archon:** replace production unwraps ([#236](https://github.com/forkwright/harmonia/issues/236)) ([d3f3321](https://github.com/forkwright/harmonia/commit/d3f332174b1a4468180402926ab17453e6dc5580))
* convert 4 #[allow] to #[expect] — unblock main kanon-lint ([#8](https://github.com/forkwright/harmonia/issues/8)) ([64aa264](https://github.com/forkwright/harmonia/commit/64aa26427f0a1d908106b2dffd659aebb13d0d5b))
* **lint:** clear the 40 kanon-lint violations blocking main CI ([#6](https://github.com/forkwright/harmonia/issues/6)) ([2051541](https://github.com/forkwright/harmonia/commit/2051541448e2a343084046e09bcfbcc9c762af30))
* **paroche:** encode kosync sha1 hashes explicitly ([#237](https://github.com/forkwright/harmonia/issues/237)) ([f481f57](https://github.com/forkwright/harmonia/commit/f481f570577bac035d67d0a026caffea995e3d69))

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
