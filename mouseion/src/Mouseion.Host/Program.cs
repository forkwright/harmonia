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
    container.Register(typeof(IBasicRepository<>), typeof(BasicRepository<>), Reuse.Singleton);
    container.Register<ISignalRMessageBroadcaster, SignalRMessageBroadcaster>(Reuse.Singleton);

    // Register MediaFile services
    container.Register<Mouseion.Core.MediaFiles.IMediaFileRepository, Mouseion.Core.MediaFiles.MediaFileRepository>(Reuse.Singleton);
    container.Register<Mouseion.Core.MediaFiles.IMediaAnalyzer, Mouseion.Core.MediaFiles.MediaAnalyzer>(Reuse.Singleton);

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
    var dbFactory = app.Services.GetRequiredService<IDbFactory>();
    var mainDb = dbFactory.Create(MigrationType.Main);
    var logDb = dbFactory.Create(MigrationType.Log);
    Log.Information("Database initialized");

    // Configure middleware pipeline
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
