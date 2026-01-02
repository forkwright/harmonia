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

    // Register metadata providers
    container.Register<Mouseion.Common.Http.IHttpClient, Mouseion.Common.Http.HttpClient>(Reuse.Singleton);
    container.Register<Mouseion.Core.MetadataSource.ResilientMetadataClient>(Reuse.Singleton);
    container.Register<Mouseion.Core.MetadataSource.IProvideBookInfo, Mouseion.Core.MetadataSource.BookInfoProxy>(Reuse.Singleton);
    container.Register<Mouseion.Core.MetadataSource.IProvideAudiobookInfo, Mouseion.Core.MetadataSource.AudiobookInfoProxy>(Reuse.Singleton);
    container.Register<Mouseion.Core.MetadataSource.IProvideMusicInfo, Mouseion.Core.MetadataSource.MusicBrainzInfoProxy>(Reuse.Singleton);
    container.Register<Mouseion.Core.MetadataSource.IProvideMovieInfo, Mouseion.Core.MetadataSource.TmdbInfoProxy>(Reuse.Singleton);

    // Register indexers
    container.Register<Mouseion.Core.Indexers.MyAnonamouse.MyAnonamouseSettings>(Reuse.Singleton);
    container.Register<Mouseion.Core.Indexers.MyAnonamouse.MyAnonamouseIndexer>(Reuse.Singleton);

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
    builder.Services.AddEndpointsApiExplorer();
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
