using System.Data;

namespace Mouseion.Core.Datastore;

public interface IDatabase
{
    IDbConnection OpenConnection();
    Version Version { get; }
    int Migration { get; }
    DatabaseType DatabaseType { get; }
    void Vacuum();
}
