# Tech debt audit: 2026-02-21

## Resolved in this PR (#191)

| Issue | Fix | Commit |
|-------|-----|--------|
| Orphaned `OidcState.cs` + `OidcStateRepository.cs` | Removed (duplicates of OidcAuthState in OidcProvider.cs) | 2896668 |
| Dead `Download/Deduplication/` directory | Removed (duplicate of Indexers/Deduplication, never wired) | 2896668 |
| `LevenstheinExtensions` typo | Renamed to `LevenshteinExtensions` (file + class) | 2896668 |
| `NewsControllerTests.cs` duplicate | Removed (split into ArticlesControllerTests + FeedsControllerTests) | c209385 |
| `BulkUpdateRequest.Ids` wrong property | Fixed to use `Items` (List\<BulkUpdateItem\>) | 4d8d4a2 |

## Known issues (not fixed, intentional or low priority)

### Duplicate MusicQualityParser
- `src/Mouseion.Core/Music/MusicQualityParser.cs`: instance-based, registered in DI
- `src/Mouseion.Core/Parser/MusicQualityParser.cs`: static class, called directly from `MusicFileAnalyzer`
- **Impact:** Confusing, two implementations of same logic with different patterns
- **Fix:** Consolidate into one. The instance-based version is preferred (testable via DI), but MusicFileAnalyzer needs refactoring to use DI injection instead of static calls
- **Risk:** Medium; touching parser code could affect quality detection

### Namespace/folder mismatches (intentional)
- `Notifications/Messages/*.cs` use `Mouseion.Core.Notifications` namespace (parent) despite living in subfolder
- `Serializer/System.Text.Json/` and `Serializer/Newtonsoft.Json/` use `Mouseion.Core.Serializer`
- These are style choices, not bugs; files grouped in folders for organization but share parent namespace for API simplicity

### Upstream URL references
- `MouseionCloudRequestBuilder.cs` still points to `radarr.servarr.com` and `api.radarr.video`
- These are real upstream API endpoints for metadata; cannot be changed until Mouseion has its own metadata service
- `PathExtensions.cs` has `DB_OLD = "radarr.db"`: migration backward compat, keep

### 83 Build warnings (pre-existing)
- CS8600/CS8602/CS8603/CS8604/CS8618/CS8625: nullable reference warnings throughout `Mouseion.Common`
- xUnit2009: "use Assert.StartsWith" style warnings in tests
- All pre-existing, not introduced by this PR

### 27 Controllers without tests
Newly added 18 test files; 27 controllers still untested:
`PodcastEpisodes, Facets, LibraryStatistics, WebcomicEpisodes, IndexerHealth, Rename, MovieStatistics, Calendar, MovieImport, BookSeries, Blocklist, ComicIssues, MangaChapters, BookStatistics, MusicScan, SeriesStatistics, MediaFiles, Chapters, AudiobookStatistics, AlbumVersions, AlbumStatistics, MediaSync, Auth, ImportListExclusion, Trakt, MAL, Tracks`

### Unused interfaces (13)
Dead interfaces that compile but have no consumers outside their defining file:
`IFileSystemLookupService, IProvidePidFile, IMouseionCloudRequestBuilder, IConsoleService, IPlatformInfo, IServiceFactory, IDebounceManager, IRSSFeedParser, IPodcastFileRepository, ISeasonRepository, ISceneMappingService, ITVDBProxy, IOidcStateRepository`

Most are from the upstream Radarr fork; keeping for now as they may be needed when features are implemented.

### Single TODO
`MinimumQualitySpecification.cs:49`: "When QualityProfiles are implemented, retrieve minimum from movie.QualityProfileId"

## Recommendations

1. **Next test sprint:** Cover the 27 untested controllers (Spec 02 ongoing)
2. **Nullable reference audit:** Enable `<Nullable>enable</Nullable>` with `<WarningsAsErrors>` to force fixes (estimated 2-3 hours)
3. **MusicQualityParser consolidation:** Replace static with DI-injected instance (30 min, low risk)
4. **Dead interface cleanup:** Remove the 13 unused interfaces; quick but coordinate with upstream tracking
