using Mouseion.Common.EnvironmentInfo;

namespace Mouseion.Core.Datastore;

public interface IConnectionStringFactory
{
    DatabaseConnectionInfo MainDbConnection { get; }
    DatabaseConnectionInfo LogDbConnection { get; }
    string GetDatabasePath(string connectionString);
}

// Connection string factory for SQLite/PostgreSQL
public class ConnectionStringFactory : IConnectionStringFactory
{
    private readonly IAppFolderInfo _appFolderInfo;

    public ConnectionStringFactory(IAppFolderInfo appFolderInfo)
    {
        _appFolderInfo = appFolderInfo;
    }

    public DatabaseConnectionInfo MainDbConnection =>
        GetConnection("mouseion.db", DatabaseType.SQLite);

    public DatabaseConnectionInfo LogDbConnection =>
        GetConnection("logs.db", DatabaseType.SQLite);

    public string GetDatabasePath(string connectionString)
    {
        var parts = connectionString.Split(';');
        foreach (var part in parts)
        {
            if (part.Trim().StartsWith("Data Source=", StringComparison.OrdinalIgnoreCase))
            {
                return part.Split('=')[1].Trim();
            }
        }

        return string.Empty;
    }

    private DatabaseConnectionInfo GetConnection(string filename, DatabaseType type)
    {
        var dbPath = Path.Combine(_appFolderInfo.AppDataFolder, filename);
        return new DatabaseConnectionInfo
        {
            DatabaseType = type,
            ConnectionString = $"Data Source={dbPath}"
        };
    }
}
