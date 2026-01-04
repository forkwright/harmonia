// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using DryIoc;
using DryIoc.Microsoft.DependencyInjection;
using Mouseion.Api.Security;
using Mouseion.Common.EnvironmentInfo;
using Mouseion.Common.Instrumentation;
using Mouseion.Core.Datastore;
using Mouseion.Core.Datastore.Migration.Framework;
using Mouseion.SignalR;
using Serilog;
using Serilog.Events;

// Early console-only logging for startup diagnostics
SerilogConfiguration.InitializeConsoleOnly(LogEventLevel.Debug);

try
{
    Log.Information("Mouseion starting up...");

    // Parse startup context from command-line args
    var startupContext = new StartupContext(args);

    // Create DryIoc container with Mouseion rules
    var container = new Container(rules => rules
        .WithConcreteTypeDynamicRegistrations()
        .WithAutoConcreteTypeResolution()
        .WithDefaultReuse(Reuse.Singleton));

    container.RegisterInstance<IStartupContext>(startupContext);

    // Register Serilog logger
    container.RegisterDelegate<Serilog.ILogger>(r => Log.Logger, Reuse.Singleton);

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

    // Register blocklist services
    container.Register<Mouseion.Core.Blocklisting.IBlocklistRepository, Mouseion.Core.Blocklisting.BlocklistRepository>(Reuse.Singleton);
    container.Register<Mouseion.Core.Blocklisting.IBlocklistService, Mouseion.Core.Blocklisting.BlocklistService>(Reuse.Singleton);

    // Register history services
    container.Register<Mouseion.Core.History.IMediaItemHistoryRepository, Mouseion.Core.History.MediaItemHistoryRepository>(Reuse.Singleton);
    container.Register<Mouseion.Core.History.IMediaItemHistoryService, Mouseion.Core.History.MediaItemHistoryService>(Reuse.Singleton);

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
    container.RegisterDelegate<IEnumerable<Mouseion.Core.HealthCheck.IProvideHealthCheck>>(r => new[]
    {
        r.Resolve<Mouseion.Core.HealthCheck.IProvideHealthCheck>(serviceKey: "RootFolder"),
        r.Resolve<Mouseion.Core.HealthCheck.IProvideHealthCheck>(serviceKey: "DiskSpace")
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

    // Create ASP.NET Core builder
    var builder = WebApplication.CreateBuilder(args);

    // Use DryIoc as service provider
    builder.Host.UseServiceProviderFactory(new DryIocServiceProviderFactory(container));

    // Add security services
    builder.Services.AddAuthentication(Mouseion.Api.Security.ApiKeyAuthenticationOptions.DefaultScheme)
        .AddScheme<Mouseion.Api.Security.ApiKeyAuthenticationOptions, Mouseion.Api.Security.ApiKeyAuthenticationHandler>(
            Mouseion.Api.Security.ApiKeyAuthenticationOptions.DefaultScheme,
            options => options.ApiKey = builder.Configuration["ApiKey"] ?? string.Empty);

    builder.Services.AddAuthorization();

    // Add ASP.NET Core services
    builder.Services.AddControllers();
    builder.Services.AddSignalR();
    builder.Services.AddMouseionTelemetry();
    builder.Services.AddMemoryCache();
    builder.Services.AddHostedService<Mouseion.Core.Jobs.TaskScheduler>();
    builder.Services.AddHttpClient();
    builder.Services.AddEndpointsApiExplorer();
    builder.Services.AddHttpClient("QBittorrent");
    builder.Services.AddHostedService<Mouseion.Core.Jobs.TaskScheduler>();
    builder.Services.AddSwaggerGen(c =>
    {
        c.SwaggerDoc("v3", new Microsoft.OpenApi.Models.OpenApiInfo
        {
            Title = "Mouseion API",
            Version = "v3",
            Description = "Unified media manager for movies, books, audiobooks, music, TV, podcasts, and comics"
        });
    });

    // Configure CORS (restrictive by default)
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

    // Configure CORS (restrictive by default)
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
    app.UseSwagger();
    app.UseSwaggerUI(c =>
    {
        c.SwaggerEndpoint("/swagger/v3/swagger.json", "Mouseion API v3");
        c.RoutePrefix = "swagger";
    });
    app.UseSecurityHeaders(); // Custom security headers middleware
    app.UseHttpsRedirection();
    app.UseCors();
    app.UseRouting();
    app.UseAuthentication();
    app.UseAuthorization();

    // Map controllers and SignalR hubs
    app.MapControllers();
    app.MapHub<MessageHub>("/signalr/messages");

    Log.Information("Mouseion started successfully - listening on {Urls}", string.Join(", ", app.Urls));
    app.Run();
}
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
