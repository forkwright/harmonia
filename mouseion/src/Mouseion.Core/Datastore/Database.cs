using System.Data;
using Serilog;

namespace Mouseion.Core.Datastore;

// Database implementation with connection factory
public class Database : IDatabase
{
    private readonly string _databaseName;
    private readonly Func<IDbConnection> _connectionFactory;
    private readonly ILogger _logger;

    public Database(string databaseName, Func<IDbConnection> connectionFactory)
    {
        _databaseName = databaseName;
        _connectionFactory = connectionFactory;
        _logger = Log.ForContext<Database>();
    }

    public IDbConnection OpenConnection()
    {
        var connection = _connectionFactory();
        _logger.Debug("Opened connection to {DatabaseName}", _databaseName);
        return connection;
    }

    public Version Version => new(0, 0, 0);
    public int Migration => 0;
    public DatabaseType DatabaseType { get; set; }

    public void Vacuum()
    {
        using var connection = OpenConnection();
        using var cmd = connection.CreateCommand();

        if (DatabaseType == DatabaseType.SQLite)
        {
            cmd.CommandText = "VACUUM";
            cmd.ExecuteNonQuery();
            _logger.Information("Vacuumed {DatabaseName}", _databaseName);
        }
    }
}
