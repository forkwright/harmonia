// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using AspNetCoreRateLimit;
using DryIoc;
using DryIoc.Microsoft.DependencyInjection;
using FluentValidation;
using Mouseion.Api.Security;
using Mouseion.Common.EnvironmentInfo;
using Mouseion.Common.Instrumentation;
using Mouseion.Core.Datastore;
using Mouseion.Core.Datastore.Migration.Framework;
using Mouseion.SignalR;
using OpenTelemetry.Metrics;
using Serilog;
using Serilog.Events;

// Early console-only logging for startup diagnostics
SerilogConfiguration.InitializeConsoleOnly(LogEventLevel.Debug);

try
{
    Log.Information("Mouseion starting up...");

    // Parse startup context from command-line args
    var startupContext = new StartupContext(args);

    // Create ASP.NET Core builder
    var builder = WebApplication.CreateBuilder(args);

    // Detect environment - use default DI for tests, DryIoc for production
    var isTestEnvironment = builder.Environment.EnvironmentName == "Test";

    if (isTestEnvironment)
    {
        Log.Information("Test environment detected - using default ASP.NET Core DI");

        // Register startup context
        builder.Services.AddSingleton<IStartupContext>(startupContext);

        // Register Serilog logger
        builder.Services.AddSingleton<Serilog.ILogger>(Log.Logger);

        // Register core services
        builder.Services.AddSingleton<IAppFolderInfo, AppFolderInfo>();
        builder.Services.AddSingleton<Mouseion.Common.Disk.IDiskProvider, Mouseion.Common.Disk.DiskProvider>();
        builder.Services.AddSingleton<Mouseion.Common.Cache.ICacheManager, Mouseion.Common.Cache.CacheManager>();
        builder.Services.AddSingleton<IMigrationController, MigrationController>();
        builder.Services.AddSingleton<IConnectionStringFactory, ConnectionStringFactory>();
        builder.Services.AddSingleton<IDbFactory, DbFactory>();
        builder.Services.AddSingleton<IDatabase>(sp =>
        {
            var dbFactory = sp.GetRequiredService<IDbFactory>();
            return dbFactory.Create(MigrationType.Main);
        });
        builder.Services.AddSingleton(typeof(IBasicRepository<>), typeof(BasicRepository<>));
        builder.Services.AddSingleton<ISignalRMessageBroadcaster, SignalRMessageBroadcaster>();

        // Register MediaItem repository
        builder.Services.AddSingleton<Mouseion.Core.MediaItems.IMediaItemRepository, Mouseion.Core.MediaItems.MediaItemRepository>();

        // Register MediaFile services
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.IMediaFileRepository, Mouseion.Core.MediaFiles.MediaFileRepository>();
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.IMediaAnalyzer, Mouseion.Core.MediaFiles.MediaAnalyzer>();
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.MediaInfo.IMediaInfoService, Mouseion.Core.MediaFiles.MediaInfo.MediaInfoService>();
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.MediaInfo.IUpdateMediaInfoService, Mouseion.Core.MediaFiles.MediaInfo.UpdateMediaInfoService>();

        // Register book/audiobook repositories
        builder.Services.AddSingleton<Mouseion.Core.Authors.IAuthorRepository, Mouseion.Core.Authors.AuthorRepository>();
        builder.Services.AddSingleton<Mouseion.Core.BookSeries.IBookSeriesRepository, Mouseion.Core.BookSeries.BookSeriesRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Books.IBookRepository, Mouseion.Core.Books.BookRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Audiobooks.IAudiobookRepository, Mouseion.Core.Audiobooks.AudiobookRepository>();

        // Register book/audiobook services
        builder.Services.AddSingleton<Mouseion.Core.Authors.IAddAuthorService, Mouseion.Core.Authors.AddAuthorService>();
        builder.Services.AddSingleton<Mouseion.Core.Books.IAddBookService, Mouseion.Core.Books.AddBookService>();
        builder.Services.AddSingleton<Mouseion.Core.Audiobooks.IAddAudiobookService, Mouseion.Core.Audiobooks.AddAudiobookService>();
        builder.Services.AddSingleton<Mouseion.Core.Books.IBookStatisticsService, Mouseion.Core.Books.BookStatisticsService>();
        builder.Services.AddSingleton<Mouseion.Core.Audiobooks.IAudiobookStatisticsService, Mouseion.Core.Audiobooks.AudiobookStatisticsService>();

        // Register music repositories
        builder.Services.AddSingleton<Mouseion.Core.Music.IArtistRepository, Mouseion.Core.Music.ArtistRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Music.IAlbumRepository, Mouseion.Core.Music.AlbumRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Music.ITrackRepository, Mouseion.Core.Music.TrackRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Music.IMusicFileRepository, Mouseion.Core.Music.MusicFileRepository>();

        // Register music services
        builder.Services.AddSingleton<Mouseion.Core.Music.IAddArtistService, Mouseion.Core.Music.AddArtistService>();
        builder.Services.AddSingleton<Mouseion.Core.Music.IAddAlbumService, Mouseion.Core.Music.AddAlbumService>();
        builder.Services.AddSingleton<Mouseion.Core.Music.IAddTrackService, Mouseion.Core.Music.AddTrackService>();
        builder.Services.AddSingleton<Mouseion.Core.Music.IArtistStatisticsService, Mouseion.Core.Music.ArtistStatisticsService>();
        builder.Services.AddSingleton<Mouseion.Core.Music.IAlbumStatisticsService, Mouseion.Core.Music.AlbumStatisticsService>();
        builder.Services.AddSingleton<Mouseion.Core.Music.IAlbumVersionsService, Mouseion.Core.Music.AlbumVersionsService>();
        builder.Services.AddSingleton<Mouseion.Core.Music.IAudioAnalysisService, Mouseion.Core.Music.AudioAnalysisService>();
        builder.Services.AddSingleton<Mouseion.Core.Music.IMusicQualityParser, Mouseion.Core.Music.MusicQualityParser>();
        builder.Services.AddSingleton<Mouseion.Core.Music.IAcoustIDService, Mouseion.Core.Music.AcoustIDService>();
        builder.Services.AddSingleton<Mouseion.Core.Music.IMusicReleaseMonitoringService, Mouseion.Core.Music.MusicReleaseMonitoringService>();
        builder.Services.AddSingleton<Mouseion.Core.Music.ITrackSearchService, Mouseion.Core.Music.TrackSearchService>();

        // Register audio analysis services
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.Audio.IDynamicRangeAnalyzer, Mouseion.Core.MediaFiles.Audio.DynamicRangeAnalyzer>();
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.Audio.IAudioFileAnalyzer, Mouseion.Core.MediaFiles.Audio.AudioFileAnalyzer>();

        // Register library filtering services
        builder.Services.AddSingleton<Mouseion.Core.Filtering.IFilterQueryBuilder, Mouseion.Core.Filtering.FilterQueryBuilder>();
        builder.Services.AddSingleton<Mouseion.Core.Library.ILibraryFilterService, Mouseion.Core.Library.LibraryFilterService>();
        builder.Services.AddSingleton<Mouseion.Core.Library.IUnifiedLibraryStatisticsService, Mouseion.Core.Library.UnifiedLibraryStatisticsService>();

        // Register tag services
        builder.Services.AddSingleton<Mouseion.Core.Tags.ITagRepository, Mouseion.Core.Tags.TagRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Tags.ITagService, Mouseion.Core.Tags.TagService>();

        // Register auto-tagging services
        builder.Services.AddSingleton<Mouseion.Core.Tags.AutoTagging.IAutoTaggingRuleRepository, Mouseion.Core.Tags.AutoTagging.AutoTaggingRuleRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Tags.AutoTagging.IAutoTaggingService, Mouseion.Core.Tags.AutoTagging.AutoTaggingService>();

        // Register root folder services
        builder.Services.AddSingleton<Mouseion.Core.RootFolders.IRootFolderRepository, Mouseion.Core.RootFolders.RootFolderRepository>();
        builder.Services.AddSingleton<Mouseion.Core.RootFolders.IRootFolderService, Mouseion.Core.RootFolders.RootFolderService>();

        // Register file scanning services
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.IDiskScanService, Mouseion.Core.MediaFiles.DiskScanService>();
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.IMusicFileAnalyzer, Mouseion.Core.MediaFiles.MusicFileAnalyzer>();
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.IMusicFileScanner, Mouseion.Core.MediaFiles.MusicFileScanner>();

        // Register import services
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.Import.Aggregation.IAggregationService, Mouseion.Core.MediaFiles.Import.Aggregation.AggregationService>();
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.Import.IImportDecisionMaker, Mouseion.Core.MediaFiles.Import.ImportDecisionMaker>();
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.Import.IImportApprovedFiles, Mouseion.Core.MediaFiles.Import.ImportApprovedFiles>();

        // Register import specifications
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.Import.Specifications.HasAudioTrackSpecification>();
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.Import.Specifications.AlreadyImportedSpecification>();
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.Import.Specifications.MinimumQualitySpecification>();
        builder.Services.AddSingleton<Mouseion.Core.MediaFiles.Import.Specifications.UpgradeSpecification>();
        builder.Services.AddSingleton<IEnumerable<Mouseion.Core.MediaFiles.Import.IImportSpecification>>(sp => new Mouseion.Core.MediaFiles.Import.IImportSpecification[]
        {
            sp.GetRequiredService<Mouseion.Core.MediaFiles.Import.Specifications.HasAudioTrackSpecification>(),
            sp.GetRequiredService<Mouseion.Core.MediaFiles.Import.Specifications.AlreadyImportedSpecification>(),
            sp.GetRequiredService<Mouseion.Core.MediaFiles.Import.Specifications.MinimumQualitySpecification>(),
            sp.GetRequiredService<Mouseion.Core.MediaFiles.Import.Specifications.UpgradeSpecification>()
        });

        // Register movie repositories
        builder.Services.AddSingleton<Mouseion.Core.Movies.IMovieRepository, Mouseion.Core.Movies.MovieRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Movies.IMovieFileRepository, Mouseion.Core.Movies.MovieFileRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Movies.ICollectionRepository, Mouseion.Core.Movies.CollectionRepository>();

        // Register movie services
        builder.Services.AddSingleton<Mouseion.Core.Movies.IAddMovieService, Mouseion.Core.Movies.AddMovieService>();
        builder.Services.AddSingleton<Mouseion.Core.Movies.IAddCollectionService, Mouseion.Core.Movies.AddCollectionService>();
        builder.Services.AddSingleton<Mouseion.Core.Movies.IMovieStatisticsService, Mouseion.Core.Movies.MovieStatisticsService>();
        builder.Services.AddSingleton<Mouseion.Core.Movies.ICollectionStatisticsService, Mouseion.Core.Movies.CollectionStatisticsService>();
        builder.Services.AddSingleton<Mouseion.Core.Movies.Organization.IFileOrganizationService, Mouseion.Core.Movies.Organization.FileOrganizationService>();

        // Register blocklist services
        builder.Services.AddSingleton<Mouseion.Core.Blocklisting.IBlocklistRepository, Mouseion.Core.Blocklisting.BlocklistRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Blocklisting.IBlocklistService, Mouseion.Core.Blocklisting.BlocklistService>();

        // Register history services
        builder.Services.AddSingleton<Mouseion.Core.History.IMediaItemHistoryRepository, Mouseion.Core.History.MediaItemHistoryRepository>();
        builder.Services.AddSingleton<Mouseion.Core.History.IMediaItemHistoryService, Mouseion.Core.History.MediaItemHistoryService>();

        // Register progress tracking and session management
        builder.Services.AddSingleton<Mouseion.Core.Progress.IMediaProgressRepository, Mouseion.Core.Progress.MediaProgressRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Progress.IPlaybackSessionRepository, Mouseion.Core.Progress.PlaybackSessionRepository>();

        // Register metadata providers
        builder.Services.AddSingleton<Mouseion.Common.Http.IHttpClient, Mouseion.Common.Http.HttpClient>();
        builder.Services.AddSingleton<Mouseion.Core.MetadataSource.ResilientMetadataClient>();
        builder.Services.AddSingleton<Mouseion.Core.MetadataSource.IProvideBookInfo, Mouseion.Core.MetadataSource.BookInfoProxy>();
        builder.Services.AddSingleton<Mouseion.Core.MetadataSource.IProvideAudiobookInfo, Mouseion.Core.MetadataSource.AudiobookInfoProxy>();
        builder.Services.AddSingleton<Mouseion.Core.MetadataSource.IProvideMusicInfo, Mouseion.Core.MetadataSource.MusicBrainzInfoProxy>();
        builder.Services.AddSingleton<Mouseion.Core.MetadataSource.IProvideMovieInfo, Mouseion.Core.MetadataSource.TmdbInfoProxy>();

        // Register media cover services
        builder.Services.AddSingleton<Mouseion.Core.MediaCovers.IImageResizer, Mouseion.Core.MediaCovers.ImageResizer>();
        builder.Services.AddSingleton<Mouseion.Core.MediaCovers.ICoverExistsSpecification, Mouseion.Core.MediaCovers.CoverExistsSpecification>();
        builder.Services.AddSingleton<Mouseion.Core.MediaCovers.IMediaCoverProxy, Mouseion.Core.MediaCovers.MediaCoverProxy>();
        builder.Services.AddSingleton<Mouseion.Core.MediaCovers.IMediaCoverService, Mouseion.Core.MediaCovers.MediaCoverService>();

        // Register subtitle services
        builder.Services.AddSingleton<Mouseion.Core.Subtitles.IOpenSubtitlesProxy, Mouseion.Core.Subtitles.OpenSubtitlesProxy>();
        builder.Services.AddSingleton<Mouseion.Core.Subtitles.ISubtitleService, Mouseion.Core.Subtitles.SubtitleService>();

        // Register import lists
        builder.Services.AddSingleton<Mouseion.Core.ImportLists.IImportListRepository, Mouseion.Core.ImportLists.ImportListRepository>();
        builder.Services.AddSingleton<Mouseion.Core.ImportLists.ImportExclusions.IImportListExclusionRepository, Mouseion.Core.ImportLists.ImportExclusions.ImportListExclusionRepository>();
        builder.Services.AddSingleton<Mouseion.Core.ImportLists.ImportExclusions.IImportListExclusionService, Mouseion.Core.ImportLists.ImportExclusions.ImportListExclusionService>();
        builder.Services.AddSingleton<Mouseion.Core.ImportLists.IImportListFactory, Mouseion.Core.ImportLists.ImportListFactory>();
        builder.Services.AddSingleton<Mouseion.Core.ImportLists.IImportListSyncService, Mouseion.Core.ImportLists.ImportListSyncService>();
        builder.Services.AddSingleton<Mouseion.Core.ImportLists.TMDb.TMDbPopularMovies>();
        builder.Services.AddSingleton<Mouseion.Core.ImportLists.TMDb.TMDbTrendingMovies>();
        builder.Services.AddSingleton<Mouseion.Core.ImportLists.TMDb.TMDbUpcomingMovies>();
        builder.Services.AddSingleton<Mouseion.Core.ImportLists.TMDb.TMDbNowPlayingMovies>();
        builder.Services.AddSingleton<Mouseion.Core.ImportLists.RSS.RssImport>();
        builder.Services.AddSingleton<Mouseion.Core.ImportLists.Custom.CustomList>();
        builder.Services.AddSingleton<IEnumerable<Mouseion.Core.ImportLists.IImportList>>(sp => new Mouseion.Core.ImportLists.IImportList[]
        {
            sp.GetRequiredService<Mouseion.Core.ImportLists.TMDb.TMDbPopularMovies>(),
            sp.GetRequiredService<Mouseion.Core.ImportLists.TMDb.TMDbTrendingMovies>(),
            sp.GetRequiredService<Mouseion.Core.ImportLists.TMDb.TMDbUpcomingMovies>(),
            sp.GetRequiredService<Mouseion.Core.ImportLists.TMDb.TMDbNowPlayingMovies>(),
            sp.GetRequiredService<Mouseion.Core.ImportLists.RSS.RssImport>(),
            sp.GetRequiredService<Mouseion.Core.ImportLists.Custom.CustomList>()
        });

        // Register indexers
        builder.Services.AddSingleton<Mouseion.Core.Indexers.MyAnonamouse.MyAnonamouseSettings>();
        builder.Services.AddSingleton<Mouseion.Core.Indexers.MyAnonamouse.MyAnonamouseIndexer>();
        builder.Services.AddSingleton<Mouseion.Core.Indexers.Gazelle.GazelleSettings>();
        builder.Services.AddSingleton<Mouseion.Core.Indexers.Gazelle.GazelleParser>();
        builder.Services.AddSingleton<Mouseion.Core.Indexers.Gazelle.GazelleIndexer>();
        builder.Services.AddSingleton<Mouseion.Core.Indexers.Torznab.TorznabSettings>();
        builder.Services.AddSingleton<Mouseion.Core.Indexers.Torznab.TorznabMusicIndexer>();

        // Register health checks
        builder.Services.AddSingleton<Mouseion.Core.HealthCheck.IHealthCheckService, Mouseion.Core.HealthCheck.HealthCheckService>();
        builder.Services.AddSingleton<Mouseion.Core.HealthCheck.Checks.RootFolderCheck>();
        builder.Services.AddSingleton<Mouseion.Core.HealthCheck.Checks.DiskSpaceCheck>();
        builder.Services.AddSingleton<Mouseion.Core.HealthCheck.Checks.NewsFeedHealthCheck>();
        builder.Services.AddSingleton<Mouseion.Core.HealthCheck.Checks.MangaLibraryHealthCheck>();
        builder.Services.AddSingleton<Mouseion.Core.HealthCheck.Checks.WebcomicLibraryHealthCheck>();
        builder.Services.AddSingleton<IEnumerable<Mouseion.Core.HealthCheck.IProvideHealthCheck>>(sp => new Mouseion.Core.HealthCheck.IProvideHealthCheck[]
        {
            sp.GetRequiredService<Mouseion.Core.HealthCheck.Checks.RootFolderCheck>(),
            sp.GetRequiredService<Mouseion.Core.HealthCheck.Checks.DiskSpaceCheck>(),
            sp.GetRequiredService<Mouseion.Core.HealthCheck.Checks.NewsFeedHealthCheck>(),
            sp.GetRequiredService<Mouseion.Core.HealthCheck.Checks.MangaLibraryHealthCheck>(),
            sp.GetRequiredService<Mouseion.Core.HealthCheck.Checks.WebcomicLibraryHealthCheck>()
        });

        // Register housekeeping tasks
        builder.Services.AddSingleton<Mouseion.Core.Housekeeping.Tasks.CleanupUnusedTags>();
        builder.Services.AddSingleton<Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedBlocklist>();
        builder.Services.AddSingleton<Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedMediaFiles>();
        builder.Services.AddSingleton<Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedImportListItems>();
        builder.Services.AddSingleton<Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedMovieCollections>();
        builder.Services.AddSingleton<Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedBookSeries>();
        builder.Services.AddSingleton<Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedAuthors>();
        builder.Services.AddSingleton<Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedArtists>();
        builder.Services.AddSingleton<Mouseion.Core.Housekeeping.Tasks.TrimLogEntries>();
        builder.Services.AddSingleton<Mouseion.Core.Housekeeping.Tasks.VacuumLogDatabase>();
        builder.Services.AddSingleton<IEnumerable<Mouseion.Core.Housekeeping.IHousekeepingTask>>(sp => new Mouseion.Core.Housekeeping.IHousekeepingTask[]
        {
            sp.GetRequiredService<Mouseion.Core.Housekeeping.Tasks.CleanupUnusedTags>(),
            sp.GetRequiredService<Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedBlocklist>(),
            sp.GetRequiredService<Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedMediaFiles>(),
            sp.GetRequiredService<Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedImportListItems>(),
            sp.GetRequiredService<Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedMovieCollections>(),
            sp.GetRequiredService<Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedBookSeries>(),
            sp.GetRequiredService<Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedAuthors>(),
            sp.GetRequiredService<Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedArtists>(),
            sp.GetRequiredService<Mouseion.Core.Housekeeping.Tasks.TrimLogEntries>(),
            sp.GetRequiredService<Mouseion.Core.Housekeeping.Tasks.VacuumLogDatabase>()
        });

        // Register scheduled tasks
        builder.Services.AddSingleton<Mouseion.Core.Jobs.Tasks.HealthCheckTask>();
        builder.Services.AddSingleton<Mouseion.Core.Jobs.Tasks.DiskScanTask>();
        builder.Services.AddSingleton<Mouseion.Core.Housekeeping.HousekeepingScheduler>();
        builder.Services.AddSingleton<IEnumerable<Mouseion.Core.Jobs.IScheduledTask>>(sp => new Mouseion.Core.Jobs.IScheduledTask[]
        {
            sp.GetRequiredService<Mouseion.Core.Jobs.Tasks.HealthCheckTask>(),
            sp.GetRequiredService<Mouseion.Core.Jobs.Tasks.DiskScanTask>(),
            sp.GetRequiredService<Mouseion.Core.Housekeeping.HousekeepingScheduler>()
        });

        // Register system info
        builder.Services.AddSingleton<Mouseion.Core.SystemInfo.ISystemService, Mouseion.Core.SystemInfo.SystemService>();

        // Register security services
        builder.Services.AddSingleton<Mouseion.Common.Security.IPathValidator, Mouseion.Common.Security.PathValidator>();

        // Register crypto services
        builder.Services.AddSingleton<Mouseion.Common.Crypto.IHashProvider, Mouseion.Common.Crypto.HashProvider>();

        // Register notification services
        builder.Services.AddSingleton<Mouseion.Core.Notifications.INotificationRepository, Mouseion.Core.Notifications.NotificationRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Notifications.INotificationFactory, Mouseion.Core.Notifications.NotificationFactory>();
        builder.Services.AddSingleton<Mouseion.Core.Notifications.INotificationService, Mouseion.Core.Notifications.NotificationService>();

        // Register news services
        builder.Services.AddSingleton<Mouseion.Core.News.INewsFeedRepository, Mouseion.Core.News.NewsFeedRepository>();
        builder.Services.AddSingleton<Mouseion.Core.News.INewsArticleRepository, Mouseion.Core.News.NewsArticleRepository>();
        builder.Services.AddSingleton<Mouseion.Core.News.RSS.INewsFeedParser, Mouseion.Core.News.RSS.NewsFeedParser>();
        builder.Services.AddSingleton<Mouseion.Core.News.IAddNewsFeedService, Mouseion.Core.News.AddNewsFeedService>();
        builder.Services.AddSingleton<Mouseion.Core.News.IRefreshNewsFeedService, Mouseion.Core.News.RefreshNewsFeedService>();

        // Register manga services
        builder.Services.AddSingleton<Mouseion.Core.Manga.IMangaSeriesRepository, Mouseion.Core.Manga.MangaSeriesRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Manga.IMangaChapterRepository, Mouseion.Core.Manga.MangaChapterRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Manga.MangaDex.IMangaDexClient, Mouseion.Core.Manga.MangaDex.MangaDexClient>();
        builder.Services.AddSingleton<Mouseion.Core.Manga.AniList.IAniListClient, Mouseion.Core.Manga.AniList.AniListClient>();
        builder.Services.AddSingleton<Mouseion.Core.Manga.IAddMangaSeriesService, Mouseion.Core.Manga.AddMangaSeriesService>();
        builder.Services.AddSingleton<Mouseion.Core.Manga.IRefreshMangaSeriesService, Mouseion.Core.Manga.RefreshMangaSeriesService>();

        // Register webcomic services
        builder.Services.AddSingleton<Mouseion.Core.Webcomic.IWebcomicSeriesRepository, Mouseion.Core.Webcomic.WebcomicSeriesRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Webcomic.IWebcomicEpisodeRepository, Mouseion.Core.Webcomic.WebcomicEpisodeRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Webcomic.IAddWebcomicSeriesService, Mouseion.Core.Webcomic.AddWebcomicSeriesService>();

        // Register comic services
        builder.Services.AddSingleton<Mouseion.Core.Comic.IComicSeriesRepository, Mouseion.Core.Comic.ComicSeriesRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Comic.IComicIssueRepository, Mouseion.Core.Comic.ComicIssueRepository>();
        builder.Services.AddSingleton<Mouseion.Core.Comic.ComicVine.IComicVineClient, Mouseion.Core.Comic.ComicVine.ComicVineClient>();
        builder.Services.AddSingleton<Mouseion.Core.Comic.IAddComicSeriesService, Mouseion.Core.Comic.AddComicSeriesService>();
        builder.Services.AddSingleton<Mouseion.Core.Comic.IRefreshComicSeriesService, Mouseion.Core.Comic.RefreshComicSeriesService>();

        // Register bulk operations service
        builder.Services.AddSingleton<Mouseion.Core.Bulk.IBulkOperationsService, Mouseion.Core.Bulk.BulkOperationsService>();
    }
    else
    {
        Log.Information("Production environment - using DryIoc container");

        // Create DryIoc container
        var container = new Container(rules => rules
            .WithAutoConcreteTypeResolution()
            .With(Made.Of(FactoryMethod.ConstructorWithResolvableArguments)));

        // Register startup context
        container.RegisterInstance<IStartupContext>(startupContext);

        // Register Serilog logger
        container.RegisterInstance<Serilog.ILogger>(Log.Logger);

        // Register core services
        container.Register<IAppFolderInfo, AppFolderInfo>(Reuse.Singleton);
        container.Register<Mouseion.Common.Disk.IDiskProvider, Mouseion.Common.Disk.DiskProvider>(Reuse.Singleton);
        container.Register<Mouseion.Common.Cache.ICacheManager, Mouseion.Common.Cache.CacheManager>(Reuse.Singleton);
        container.Register<IMigrationController, MigrationController>(Reuse.Singleton);
        container.Register<IConnectionStringFactory, ConnectionStringFactory>(Reuse.Singleton);
        container.Register<IDbFactory, DbFactory>(Reuse.Singleton);
        container.RegisterDelegate<IDatabase>(r =>
        {
            var dbFactory = r.Resolve<IDbFactory>();
            return dbFactory.Create(MigrationType.Main);
        }, Reuse.Singleton);
        container.Register(typeof(IBasicRepository<>), typeof(BasicRepository<>), Reuse.Singleton);
        container.Register<ISignalRMessageBroadcaster, SignalRMessageBroadcaster>(Reuse.Singleton);

        // Register MediaItem repository
        container.Register<Mouseion.Core.MediaItems.IMediaItemRepository, Mouseion.Core.MediaItems.MediaItemRepository>(Reuse.Singleton);

        // Register MediaFile services
        container.Register<Mouseion.Core.MediaFiles.IMediaFileRepository, Mouseion.Core.MediaFiles.MediaFileRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.MediaFiles.IMediaAnalyzer, Mouseion.Core.MediaFiles.MediaAnalyzer>(Reuse.Singleton);
        container.Register<Mouseion.Core.MediaFiles.MediaInfo.IMediaInfoService, Mouseion.Core.MediaFiles.MediaInfo.MediaInfoService>(Reuse.Singleton);
        container.Register<Mouseion.Core.MediaFiles.MediaInfo.IUpdateMediaInfoService, Mouseion.Core.MediaFiles.MediaInfo.UpdateMediaInfoService>(Reuse.Singleton);

        // Register book/audiobook repositories
        container.Register<Mouseion.Core.Authors.IAuthorRepository, Mouseion.Core.Authors.AuthorRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.BookSeries.IBookSeriesRepository, Mouseion.Core.BookSeries.BookSeriesRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Books.IBookRepository, Mouseion.Core.Books.BookRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Audiobooks.IAudiobookRepository, Mouseion.Core.Audiobooks.AudiobookRepository>(Reuse.Singleton);

        // Register book/audiobook services
        container.Register<Mouseion.Core.Authors.IAddAuthorService, Mouseion.Core.Authors.AddAuthorService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Books.IAddBookService, Mouseion.Core.Books.AddBookService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Audiobooks.IAddAudiobookService, Mouseion.Core.Audiobooks.AddAudiobookService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Books.IBookStatisticsService, Mouseion.Core.Books.BookStatisticsService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Audiobooks.IAudiobookStatisticsService, Mouseion.Core.Audiobooks.AudiobookStatisticsService>(Reuse.Singleton);

        // Register music repositories
        container.Register<Mouseion.Core.Music.IArtistRepository, Mouseion.Core.Music.ArtistRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Music.IAlbumRepository, Mouseion.Core.Music.AlbumRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Music.ITrackRepository, Mouseion.Core.Music.TrackRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Music.IMusicFileRepository, Mouseion.Core.Music.MusicFileRepository>(Reuse.Singleton);

        // Register music services
        container.Register<Mouseion.Core.Music.IAddArtistService, Mouseion.Core.Music.AddArtistService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Music.IAddAlbumService, Mouseion.Core.Music.AddAlbumService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Music.IAddTrackService, Mouseion.Core.Music.AddTrackService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Music.IArtistStatisticsService, Mouseion.Core.Music.ArtistStatisticsService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Music.IAlbumStatisticsService, Mouseion.Core.Music.AlbumStatisticsService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Music.IAlbumVersionsService, Mouseion.Core.Music.AlbumVersionsService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Music.IAudioAnalysisService, Mouseion.Core.Music.AudioAnalysisService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Music.IMusicQualityParser, Mouseion.Core.Music.MusicQualityParser>(Reuse.Singleton);
        container.Register<Mouseion.Core.Music.IAcoustIDService, Mouseion.Core.Music.AcoustIDService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Music.IMusicReleaseMonitoringService, Mouseion.Core.Music.MusicReleaseMonitoringService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Music.ITrackSearchService, Mouseion.Core.Music.TrackSearchService>(Reuse.Singleton);

        // Register audio analysis services
        container.Register<Mouseion.Core.MediaFiles.Audio.IDynamicRangeAnalyzer, Mouseion.Core.MediaFiles.Audio.DynamicRangeAnalyzer>(Reuse.Singleton);
        container.Register<Mouseion.Core.MediaFiles.Audio.IAudioFileAnalyzer, Mouseion.Core.MediaFiles.Audio.AudioFileAnalyzer>(Reuse.Singleton);

        // Register library filtering services
        container.Register<Mouseion.Core.Filtering.IFilterQueryBuilder, Mouseion.Core.Filtering.FilterQueryBuilder>(Reuse.Singleton);
        container.Register<Mouseion.Core.Library.ILibraryFilterService, Mouseion.Core.Library.LibraryFilterService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Library.IUnifiedLibraryStatisticsService, Mouseion.Core.Library.UnifiedLibraryStatisticsService>(Reuse.Singleton);

        // Register tag services
        container.Register<Mouseion.Core.Tags.ITagRepository, Mouseion.Core.Tags.TagRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Tags.ITagService, Mouseion.Core.Tags.TagService>(Reuse.Singleton);

        // Register root folder services
        container.Register<Mouseion.Core.RootFolders.IRootFolderRepository, Mouseion.Core.RootFolders.RootFolderRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.RootFolders.IRootFolderService, Mouseion.Core.RootFolders.RootFolderService>(Reuse.Singleton);

        // Register file scanning services
        container.Register<Mouseion.Core.MediaFiles.IDiskScanService, Mouseion.Core.MediaFiles.DiskScanService>(Reuse.Singleton);
        container.Register<Mouseion.Core.MediaFiles.IMusicFileAnalyzer, Mouseion.Core.MediaFiles.MusicFileAnalyzer>(Reuse.Singleton);
        container.Register<Mouseion.Core.MediaFiles.IMusicFileScanner, Mouseion.Core.MediaFiles.MusicFileScanner>(Reuse.Singleton);

        // Register import services
        container.Register<Mouseion.Core.MediaFiles.Import.Aggregation.IAggregationService, Mouseion.Core.MediaFiles.Import.Aggregation.AggregationService>(Reuse.Singleton);
        container.Register<Mouseion.Core.MediaFiles.Import.IImportDecisionMaker, Mouseion.Core.MediaFiles.Import.ImportDecisionMaker>(Reuse.Singleton);
        container.Register<Mouseion.Core.MediaFiles.Import.IImportApprovedFiles, Mouseion.Core.MediaFiles.Import.ImportApprovedFiles>(Reuse.Singleton);

        // Register import specifications
        container.Register<Mouseion.Core.MediaFiles.Import.IImportSpecification, Mouseion.Core.MediaFiles.Import.Specifications.HasAudioTrackSpecification>(Reuse.Singleton, serviceKey: "HasAudioTrack");
        container.Register<Mouseion.Core.MediaFiles.Import.IImportSpecification, Mouseion.Core.MediaFiles.Import.Specifications.AlreadyImportedSpecification>(Reuse.Singleton, serviceKey: "AlreadyImported");
        container.Register<Mouseion.Core.MediaFiles.Import.IImportSpecification, Mouseion.Core.MediaFiles.Import.Specifications.MinimumQualitySpecification>(Reuse.Singleton, serviceKey: "MinimumQuality");
        container.Register<Mouseion.Core.MediaFiles.Import.IImportSpecification, Mouseion.Core.MediaFiles.Import.Specifications.UpgradeSpecification>(Reuse.Singleton, serviceKey: "Upgrade");
        container.RegisterDelegate<IEnumerable<Mouseion.Core.MediaFiles.Import.IImportSpecification>>(r => new[]
        {
            r.Resolve<Mouseion.Core.MediaFiles.Import.IImportSpecification>(serviceKey: "HasAudioTrack"),
            r.Resolve<Mouseion.Core.MediaFiles.Import.IImportSpecification>(serviceKey: "AlreadyImported"),
            r.Resolve<Mouseion.Core.MediaFiles.Import.IImportSpecification>(serviceKey: "MinimumQuality"),
            r.Resolve<Mouseion.Core.MediaFiles.Import.IImportSpecification>(serviceKey: "Upgrade")
        }, Reuse.Singleton);

        // Register movie repositories
        container.Register<Mouseion.Core.Movies.IMovieRepository, Mouseion.Core.Movies.MovieRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Movies.IMovieFileRepository, Mouseion.Core.Movies.MovieFileRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Movies.ICollectionRepository, Mouseion.Core.Movies.CollectionRepository>(Reuse.Singleton);

        // Register movie services
        container.Register<Mouseion.Core.Movies.IAddMovieService, Mouseion.Core.Movies.AddMovieService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Movies.IAddCollectionService, Mouseion.Core.Movies.AddCollectionService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Movies.IMovieStatisticsService, Mouseion.Core.Movies.MovieStatisticsService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Movies.ICollectionStatisticsService, Mouseion.Core.Movies.CollectionStatisticsService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Movies.Organization.IFileOrganizationService, Mouseion.Core.Movies.Organization.FileOrganizationService>(Reuse.Singleton);

        // Register blocklist services
        container.Register<Mouseion.Core.Blocklisting.IBlocklistRepository, Mouseion.Core.Blocklisting.BlocklistRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Blocklisting.IBlocklistService, Mouseion.Core.Blocklisting.BlocklistService>(Reuse.Singleton);

        // Register history services
        container.Register<Mouseion.Core.History.IMediaItemHistoryRepository, Mouseion.Core.History.MediaItemHistoryRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.History.IMediaItemHistoryService, Mouseion.Core.History.MediaItemHistoryService>(Reuse.Singleton);

        // Register progress tracking and session management
        container.Register<Mouseion.Core.Progress.IMediaProgressRepository, Mouseion.Core.Progress.MediaProgressRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Progress.IPlaybackSessionRepository, Mouseion.Core.Progress.PlaybackSessionRepository>(Reuse.Singleton);

        // Register metadata providers
        container.Register<Mouseion.Common.Http.IHttpClient, Mouseion.Common.Http.HttpClient>(Reuse.Singleton);
        container.Register<Mouseion.Core.MetadataSource.ResilientMetadataClient>(Reuse.Singleton);
        container.Register<Mouseion.Core.MetadataSource.IProvideBookInfo, Mouseion.Core.MetadataSource.BookInfoProxy>(Reuse.Singleton);
        container.Register<Mouseion.Core.MetadataSource.IProvideAudiobookInfo, Mouseion.Core.MetadataSource.AudiobookInfoProxy>(Reuse.Singleton);
        container.Register<Mouseion.Core.MetadataSource.IProvideMusicInfo, Mouseion.Core.MetadataSource.MusicBrainzInfoProxy>(Reuse.Singleton);
        container.Register<Mouseion.Core.MetadataSource.IProvideMovieInfo, Mouseion.Core.MetadataSource.TmdbInfoProxy>(Reuse.Singleton);

        // Register media cover services
        container.Register<Mouseion.Core.MediaCovers.IImageResizer, Mouseion.Core.MediaCovers.ImageResizer>(Reuse.Singleton);
        container.Register<Mouseion.Core.MediaCovers.ICoverExistsSpecification, Mouseion.Core.MediaCovers.CoverExistsSpecification>(Reuse.Singleton);
        container.Register<Mouseion.Core.MediaCovers.IMediaCoverProxy, Mouseion.Core.MediaCovers.MediaCoverProxy>(Reuse.Singleton);
        container.Register<Mouseion.Core.MediaCovers.IMediaCoverService, Mouseion.Core.MediaCovers.MediaCoverService>(Reuse.Singleton);

        // Register subtitle services
        container.Register<Mouseion.Core.Subtitles.IOpenSubtitlesProxy, Mouseion.Core.Subtitles.OpenSubtitlesProxy>(Reuse.Singleton);
        container.Register<Mouseion.Core.Subtitles.ISubtitleService, Mouseion.Core.Subtitles.SubtitleService>(Reuse.Singleton);

        // Register import lists
        container.Register<Mouseion.Core.ImportLists.IImportListRepository, Mouseion.Core.ImportLists.ImportListRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.ImportLists.ImportExclusions.IImportListExclusionRepository, Mouseion.Core.ImportLists.ImportExclusions.ImportListExclusionRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.ImportLists.ImportExclusions.IImportListExclusionService, Mouseion.Core.ImportLists.ImportExclusions.ImportListExclusionService>(Reuse.Singleton);
        container.Register<Mouseion.Core.ImportLists.IImportListFactory, Mouseion.Core.ImportLists.ImportListFactory>(Reuse.Singleton);
        container.Register<Mouseion.Core.ImportLists.IImportListSyncService, Mouseion.Core.ImportLists.ImportListSyncService>(Reuse.Singleton);
        container.Register<Mouseion.Core.ImportLists.IImportList, Mouseion.Core.ImportLists.TMDb.TMDbPopularMovies>(Reuse.Singleton, serviceKey: "TMDbPopularMovies");
        container.Register<Mouseion.Core.ImportLists.IImportList, Mouseion.Core.ImportLists.TMDb.TMDbTrendingMovies>(Reuse.Singleton, serviceKey: "TMDbTrendingMovies");
        container.Register<Mouseion.Core.ImportLists.IImportList, Mouseion.Core.ImportLists.TMDb.TMDbUpcomingMovies>(Reuse.Singleton, serviceKey: "TMDbUpcomingMovies");
        container.Register<Mouseion.Core.ImportLists.IImportList, Mouseion.Core.ImportLists.TMDb.TMDbNowPlayingMovies>(Reuse.Singleton, serviceKey: "TMDbNowPlayingMovies");
        container.Register<Mouseion.Core.ImportLists.IImportList, Mouseion.Core.ImportLists.RSS.RssImport>(Reuse.Singleton, serviceKey: "RSSImport");
        container.Register<Mouseion.Core.ImportLists.IImportList, Mouseion.Core.ImportLists.Custom.CustomList>(Reuse.Singleton, serviceKey: "CustomList");
        container.RegisterDelegate<IEnumerable<Mouseion.Core.ImportLists.IImportList>>(r => new[]
        {
            r.Resolve<Mouseion.Core.ImportLists.IImportList>(serviceKey: "TMDbPopularMovies"),
            r.Resolve<Mouseion.Core.ImportLists.IImportList>(serviceKey: "TMDbTrendingMovies"),
            r.Resolve<Mouseion.Core.ImportLists.IImportList>(serviceKey: "TMDbUpcomingMovies"),
            r.Resolve<Mouseion.Core.ImportLists.IImportList>(serviceKey: "TMDbNowPlayingMovies"),
            r.Resolve<Mouseion.Core.ImportLists.IImportList>(serviceKey: "RSSImport"),
            r.Resolve<Mouseion.Core.ImportLists.IImportList>(serviceKey: "CustomList")
        }, Reuse.Singleton);
        // Register indexers
        container.Register<Mouseion.Core.Indexers.MyAnonamouse.MyAnonamouseSettings>(Reuse.Singleton);
        container.Register<Mouseion.Core.Indexers.MyAnonamouse.MyAnonamouseIndexer>(Reuse.Singleton);
        container.Register<Mouseion.Core.Indexers.Gazelle.GazelleSettings>(Reuse.Singleton);
        container.Register<Mouseion.Core.Indexers.Gazelle.GazelleParser>(Reuse.Singleton);
        container.Register<Mouseion.Core.Indexers.Gazelle.GazelleIndexer>(Reuse.Singleton);
        container.Register<Mouseion.Core.Indexers.Torznab.TorznabSettings>(Reuse.Singleton);
        container.Register<Mouseion.Core.Indexers.Torznab.TorznabMusicIndexer>(Reuse.Singleton);

        // Register health checks
        container.Register<Mouseion.Core.HealthCheck.IHealthCheckService, Mouseion.Core.HealthCheck.HealthCheckService>(Reuse.Singleton);
        container.Register<Mouseion.Core.HealthCheck.IProvideHealthCheck, Mouseion.Core.HealthCheck.Checks.RootFolderCheck>(Reuse.Singleton, serviceKey: "RootFolder");
        container.Register<Mouseion.Core.HealthCheck.IProvideHealthCheck, Mouseion.Core.HealthCheck.Checks.DiskSpaceCheck>(Reuse.Singleton, serviceKey: "DiskSpace");
        container.Register<Mouseion.Core.HealthCheck.IProvideHealthCheck, Mouseion.Core.HealthCheck.Checks.NewsFeedHealthCheck>(Reuse.Singleton, serviceKey: "NewsFeed");
        container.Register<Mouseion.Core.HealthCheck.IProvideHealthCheck, Mouseion.Core.HealthCheck.Checks.MangaLibraryHealthCheck>(Reuse.Singleton, serviceKey: "MangaLibrary");
        container.Register<Mouseion.Core.HealthCheck.IProvideHealthCheck, Mouseion.Core.HealthCheck.Checks.WebcomicLibraryHealthCheck>(Reuse.Singleton, serviceKey: "WebcomicLibrary");
        container.RegisterDelegate<IEnumerable<Mouseion.Core.HealthCheck.IProvideHealthCheck>>(r => new[]
        {
            r.Resolve<Mouseion.Core.HealthCheck.IProvideHealthCheck>(serviceKey: "RootFolder"),
            r.Resolve<Mouseion.Core.HealthCheck.IProvideHealthCheck>(serviceKey: "DiskSpace"),
            r.Resolve<Mouseion.Core.HealthCheck.IProvideHealthCheck>(serviceKey: "NewsFeed"),
            r.Resolve<Mouseion.Core.HealthCheck.IProvideHealthCheck>(serviceKey: "MangaLibrary"),
            r.Resolve<Mouseion.Core.HealthCheck.IProvideHealthCheck>(serviceKey: "WebcomicLibrary")
        }, Reuse.Singleton);

        // Register housekeeping tasks
        container.Register<Mouseion.Core.Housekeeping.IHousekeepingTask, Mouseion.Core.Housekeeping.Tasks.CleanupUnusedTags>(Reuse.Singleton, serviceKey: "CleanupUnusedTags");
        container.Register<Mouseion.Core.Housekeeping.IHousekeepingTask, Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedBlocklist>(Reuse.Singleton, serviceKey: "CleanupOrphanedBlocklist");
        container.Register<Mouseion.Core.Housekeeping.IHousekeepingTask, Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedMediaFiles>(Reuse.Singleton, serviceKey: "CleanupOrphanedMediaFiles");
        container.Register<Mouseion.Core.Housekeeping.IHousekeepingTask, Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedImportListItems>(Reuse.Singleton, serviceKey: "CleanupOrphanedImportListItems");
        container.Register<Mouseion.Core.Housekeeping.IHousekeepingTask, Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedMovieCollections>(Reuse.Singleton, serviceKey: "CleanupOrphanedMovieCollections");
        container.Register<Mouseion.Core.Housekeeping.IHousekeepingTask, Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedBookSeries>(Reuse.Singleton, serviceKey: "CleanupOrphanedBookSeries");
        container.Register<Mouseion.Core.Housekeeping.IHousekeepingTask, Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedAuthors>(Reuse.Singleton, serviceKey: "CleanupOrphanedAuthors");
        container.Register<Mouseion.Core.Housekeeping.IHousekeepingTask, Mouseion.Core.Housekeeping.Tasks.CleanupOrphanedArtists>(Reuse.Singleton, serviceKey: "CleanupOrphanedArtists");
        container.Register<Mouseion.Core.Housekeeping.IHousekeepingTask, Mouseion.Core.Housekeeping.Tasks.TrimLogEntries>(Reuse.Singleton, serviceKey: "TrimLogEntries");
        container.Register<Mouseion.Core.Housekeeping.IHousekeepingTask, Mouseion.Core.Housekeeping.Tasks.VacuumLogDatabase>(Reuse.Singleton, serviceKey: "VacuumLogDatabase");
        container.RegisterDelegate<IEnumerable<Mouseion.Core.Housekeeping.IHousekeepingTask>>(r => new[]
        {
            r.Resolve<Mouseion.Core.Housekeeping.IHousekeepingTask>(serviceKey: "CleanupUnusedTags"),
            r.Resolve<Mouseion.Core.Housekeeping.IHousekeepingTask>(serviceKey: "CleanupOrphanedBlocklist"),
            r.Resolve<Mouseion.Core.Housekeeping.IHousekeepingTask>(serviceKey: "CleanupOrphanedMediaFiles"),
            r.Resolve<Mouseion.Core.Housekeeping.IHousekeepingTask>(serviceKey: "CleanupOrphanedImportListItems"),
            r.Resolve<Mouseion.Core.Housekeeping.IHousekeepingTask>(serviceKey: "CleanupOrphanedMovieCollections"),
            r.Resolve<Mouseion.Core.Housekeeping.IHousekeepingTask>(serviceKey: "CleanupOrphanedBookSeries"),
            r.Resolve<Mouseion.Core.Housekeeping.IHousekeepingTask>(serviceKey: "CleanupOrphanedAuthors"),
            r.Resolve<Mouseion.Core.Housekeeping.IHousekeepingTask>(serviceKey: "CleanupOrphanedArtists"),
            r.Resolve<Mouseion.Core.Housekeeping.IHousekeepingTask>(serviceKey: "TrimLogEntries"),
            r.Resolve<Mouseion.Core.Housekeeping.IHousekeepingTask>(serviceKey: "VacuumLogDatabase")
        }, Reuse.Singleton);

        // Register scheduled tasks
        container.Register<Mouseion.Core.Jobs.IScheduledTask, Mouseion.Core.Jobs.Tasks.HealthCheckTask>(Reuse.Singleton, serviceKey: "HealthCheck");
        container.Register<Mouseion.Core.Jobs.IScheduledTask, Mouseion.Core.Jobs.Tasks.DiskScanTask>(Reuse.Singleton, serviceKey: "DiskScan");
        container.Register<Mouseion.Core.Jobs.IScheduledTask, Mouseion.Core.Housekeeping.HousekeepingScheduler>(Reuse.Singleton, serviceKey: "Housekeeping");
        container.RegisterDelegate<IEnumerable<Mouseion.Core.Jobs.IScheduledTask>>(r => new[]
        {
            r.Resolve<Mouseion.Core.Jobs.IScheduledTask>(serviceKey: "HealthCheck"),
            r.Resolve<Mouseion.Core.Jobs.IScheduledTask>(serviceKey: "DiskScan"),
            r.Resolve<Mouseion.Core.Jobs.IScheduledTask>(serviceKey: "Housekeeping")
        }, Reuse.Singleton);

        // Register system info
        container.Register<Mouseion.Core.SystemInfo.ISystemService, Mouseion.Core.SystemInfo.SystemService>(Reuse.Singleton);

        // Register security services
        container.Register<Mouseion.Common.Security.IPathValidator, Mouseion.Common.Security.PathValidator>(Reuse.Singleton);

        // Register crypto services
        container.Register<Mouseion.Common.Crypto.IHashProvider, Mouseion.Common.Crypto.HashProvider>(Reuse.Singleton);

        // Register notification services
        container.Register<Mouseion.Core.Notifications.INotificationRepository, Mouseion.Core.Notifications.NotificationRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Notifications.INotificationFactory, Mouseion.Core.Notifications.NotificationFactory>(Reuse.Singleton);
        container.Register<Mouseion.Core.Notifications.INotificationService, Mouseion.Core.Notifications.NotificationService>(Reuse.Singleton);

        // Register news services
        container.Register<Mouseion.Core.News.INewsFeedRepository, Mouseion.Core.News.NewsFeedRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.News.INewsArticleRepository, Mouseion.Core.News.NewsArticleRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.News.RSS.INewsFeedParser, Mouseion.Core.News.RSS.NewsFeedParser>(Reuse.Singleton);
        container.Register<Mouseion.Core.News.IAddNewsFeedService, Mouseion.Core.News.AddNewsFeedService>(Reuse.Singleton);
        container.Register<Mouseion.Core.News.IRefreshNewsFeedService, Mouseion.Core.News.RefreshNewsFeedService>(Reuse.Singleton);

        // Register manga services
        container.Register<Mouseion.Core.Manga.IMangaSeriesRepository, Mouseion.Core.Manga.MangaSeriesRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Manga.IMangaChapterRepository, Mouseion.Core.Manga.MangaChapterRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Manga.MangaDex.IMangaDexClient, Mouseion.Core.Manga.MangaDex.MangaDexClient>(Reuse.Singleton);
        container.Register<Mouseion.Core.Manga.AniList.IAniListClient, Mouseion.Core.Manga.AniList.AniListClient>(Reuse.Singleton);
        container.Register<Mouseion.Core.Manga.IAddMangaSeriesService, Mouseion.Core.Manga.AddMangaSeriesService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Manga.IRefreshMangaSeriesService, Mouseion.Core.Manga.RefreshMangaSeriesService>(Reuse.Singleton);

        // Register webcomic services
        container.Register<Mouseion.Core.Webcomic.IWebcomicSeriesRepository, Mouseion.Core.Webcomic.WebcomicSeriesRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Webcomic.IWebcomicEpisodeRepository, Mouseion.Core.Webcomic.WebcomicEpisodeRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Webcomic.IAddWebcomicSeriesService, Mouseion.Core.Webcomic.AddWebcomicSeriesService>(Reuse.Singleton);

        // Register comic services
        container.Register<Mouseion.Core.Comic.IComicSeriesRepository, Mouseion.Core.Comic.ComicSeriesRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Comic.IComicIssueRepository, Mouseion.Core.Comic.ComicIssueRepository>(Reuse.Singleton);
        container.Register<Mouseion.Core.Comic.ComicVine.IComicVineClient, Mouseion.Core.Comic.ComicVine.ComicVineClient>(Reuse.Singleton);
        container.Register<Mouseion.Core.Comic.IAddComicSeriesService, Mouseion.Core.Comic.AddComicSeriesService>(Reuse.Singleton);
        container.Register<Mouseion.Core.Comic.IRefreshComicSeriesService, Mouseion.Core.Comic.RefreshComicSeriesService>(Reuse.Singleton);

        // Register bulk operations service
        container.Register<Mouseion.Core.Bulk.IBulkOperationsService, Mouseion.Core.Bulk.BulkOperationsService>(Reuse.Singleton);

        // Use DryIoc as service provider
        builder.Host.UseServiceProviderFactory(new DryIocServiceProviderFactory(container));
    }

    // Add security services
    builder.Services.AddAuthentication(Mouseion.Api.Security.ApiKeyAuthenticationOptions.DefaultScheme)
        .AddScheme<Mouseion.Api.Security.ApiKeyAuthenticationOptions, Mouseion.Api.Security.ApiKeyAuthenticationHandler>(
            Mouseion.Api.Security.ApiKeyAuthenticationOptions.DefaultScheme,
            options => options.ApiKey = builder.Configuration["ApiKey"] ?? string.Empty);

    builder.Services.AddAuthorization();

    // Add ASP.NET Core services
    builder.Services.AddControllers(options =>
    {
        // Add validation filter for automatic FluentValidation
        options.Filters.Add<Mouseion.Api.Validation.ValidationFilter>();
    });

    // Add FluentValidation (registers all validators in Mouseion.Api assembly)
    builder.Services.AddValidatorsFromAssemblyContaining<Mouseion.Api.Common.ApiProblemDetails>();
    builder.Services.AddScoped<Mouseion.Api.Validation.ValidationFilter>();

    builder.Services.AddSignalR();
    builder.Services.AddMouseionTelemetry(builder.Configuration);
    builder.Services.AddMemoryCache();

    // Skip task scheduler in test mode (background services start before database initialization)
    if (!isTestEnvironment)
    {
        builder.Services.AddHostedService<Mouseion.Core.Jobs.TaskScheduler>();
    }

    builder.Services.AddHttpClient();
    builder.Services.AddEndpointsApiExplorer();
    builder.Services.AddHttpClient("QBittorrent");
    builder.Services.AddHttpClient("OpenSubtitles", client =>
    {
        client.DefaultRequestHeaders.Add("Api-Key", builder.Configuration["OpenSubtitles:ApiKey"] ?? string.Empty);
        client.DefaultRequestHeaders.Add("User-Agent", "Mouseion v1");
    });
    builder.Services.AddSwaggerGen(c =>
    {
        c.SwaggerDoc("v3", new Microsoft.OpenApi.Models.OpenApiInfo
        {
            Title = "Mouseion API",
            Version = "v3",
            Description = "Unified media manager for movies, books, audiobooks, music, TV, podcasts, and comics"
        });
    });

    // Configure CORS (restrictive by default - requires AllowedOrigins in appsettings.json)
    builder.Services.AddCors(options =>
    {
        options.AddDefaultPolicy(policy =>
        {
            policy.WithOrigins(builder.Configuration.GetSection("AllowedOrigins").Get<string[]>() ?? Array.Empty<string>())
                  .AllowAnyMethod()
                  .AllowAnyHeader()
                  .AllowCredentials();
        });
    });

    // Configure rate limiting (DoS prevention)
    builder.Services.AddMemoryCache();
    builder.Services.Configure<AspNetCoreRateLimit.IpRateLimitOptions>(builder.Configuration.GetSection("IpRateLimiting"));
    builder.Services.Configure<AspNetCoreRateLimit.ClientRateLimitOptions>(builder.Configuration.GetSection("ClientRateLimiting"));
    builder.Services.AddInMemoryRateLimiting();
    builder.Services.AddSingleton<AspNetCoreRateLimit.IRateLimitConfiguration, AspNetCoreRateLimit.RateLimitConfiguration>();

    // Build the app
    var app = builder.Build();

    // Initialize proper logging with file output
    var appFolderInfo = app.Services.GetRequiredService<IAppFolderInfo>();
    SerilogConfiguration.Initialize(appFolderInfo, LogEventLevel.Information);

    Log.Information("Mouseion {Version} starting", BuildInfo.Version);
    Log.Information("AppData folder: {AppDataFolder}", appFolderInfo.AppDataFolder);

    // Ensure AppData directory exists
    Directory.CreateDirectory(appFolderInfo.AppDataFolder);

    // Initialize database (run migrations)
    Log.Information("Initializing database...");
    _ = app.Services.GetRequiredService<IDatabase>(); // Triggers creation and migrations via delegate
    var dbFactory = app.Services.GetRequiredService<IDbFactory>();
    _ = dbFactory.Create(MigrationType.Log); // Triggers creation and migrations
    Log.Information("Database initialized");

    // Configure middleware pipeline
    app.UseMiddleware<Mouseion.Api.Middleware.GlobalExceptionHandlerMiddleware>();
    app.UseMiddleware<Mouseion.Api.Middleware.TelemetryMiddleware>();
    app.UseSwagger();
    app.UseSwaggerUI(c =>
    {
        c.SwaggerEndpoint("/swagger/v3/swagger.json", "Mouseion API v3");
        c.RoutePrefix = "swagger";
    });
    app.UseSecurityHeaders(); // Custom security headers middleware
    app.UseHttpsRedirection();
    app.UseIpRateLimiting(); // IP-based rate limiting
    app.UseCors();
    app.UseRouting();
    app.UseAuthentication();
    app.UseClientRateLimiting(); // API key-based rate limiting (after authentication)
    app.UseAuthorization();

    // Map controllers and SignalR hubs
    app.MapControllers();
    app.MapHub<MessageHub>("/signalr/messages");

    // Expose Prometheus metrics endpoint (if enabled in configuration)
    if (builder.Configuration.GetValue("Telemetry:EnablePrometheus", true))
    {
        app.UseOpenTelemetryPrometheusScrapingEndpoint();
    }

    Log.Information("Mouseion started successfully - listening on {Urls}", string.Join(", ", app.Urls));
    app.Run();
}
// Top-level exception handler - generic Exception is appropriate here to catch any unhandled
// application errors for logging before termination. This is the final safety net.
catch (Exception ex)
{
    Log.Fatal(ex, "Mouseion terminated unexpectedly");
    throw;
}
finally
{
    SerilogConfiguration.CloseAndFlush();
}

// Make Program class accessible for integration testing
public partial class Program { }
