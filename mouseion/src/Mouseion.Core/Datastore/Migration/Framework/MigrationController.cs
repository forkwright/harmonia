using FluentMigrator.Runner;
using Microsoft.Extensions.DependencyInjection;
using Serilog;

namespace Mouseion.Core.Datastore.Migration.Framework;

public interface IMigrationController
{
    void Migrate(string connectionString, MigrationContext context, DatabaseType databaseType);
}

// Migration controller using FluentMigrator
public class MigrationController : IMigrationController
{
    private readonly ILogger _logger;

    public MigrationController()
    {
        _logger = Log.ForContext<MigrationController>();
    }

    public void Migrate(string connectionString, MigrationContext context, DatabaseType databaseType)
    {
        _logger.Information("Running {MigrationType} database migrations", context.MigrationType);

        var serviceProvider = new ServiceCollection()
            .AddFluentMigratorCore()
            .ConfigureRunner(rb => ConfigureRunner(rb, connectionString, databaseType))
            .AddLogging(lb => lb.AddSerilog())
            .BuildServiceProvider();

        var runner = serviceProvider.GetRequiredService<IMigrationRunner>();
        runner.MigrateUp();

        _logger.Information("Migrations complete for {MigrationType}", context.MigrationType);
    }

    private void ConfigureRunner(
        IMigrationRunnerBuilder runnerBuilder,
        string connectionString,
        DatabaseType databaseType)
    {
        if (databaseType == DatabaseType.SQLite)
        {
            runnerBuilder.AddSQLite().WithGlobalConnectionString(connectionString);
        }
        else
        {
            runnerBuilder.AddPostgres().WithGlobalConnectionString(connectionString);
        }

        runnerBuilder.ScanIn(typeof(MigrationController).Assembly).For.Migrations();
    }
}
