namespace Mouseion.Core.Datastore;

public class DatabaseConnectionInfo
{
    public DatabaseType DatabaseType { get; set; }
    public string ConnectionString { get; set; } = string.Empty;
}

public enum DatabaseType
{
    SQLite,
    PostgreSQL
}
