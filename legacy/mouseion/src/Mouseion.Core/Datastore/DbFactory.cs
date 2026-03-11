// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Data.Sqlite;
using Mouseion.Core.Datastore.Migration.Framework;
using Npgsql;

namespace Mouseion.Core.Datastore;

public interface IDbFactory
{
    IDatabase Create(MigrationType migrationType = MigrationType.Main);
}

// Database factory with migration support
public class DbFactory : IDbFactory
{
    private readonly IMigrationController _migrationController;
    private readonly IConnectionStringFactory _connectionStringFactory;

    public DbFactory(IMigrationController migrationController, IConnectionStringFactory connectionStringFactory)
    {
        _migrationController = migrationController;
        _connectionStringFactory = connectionStringFactory;
    }

    public IDatabase Create(MigrationType migrationType = MigrationType.Main)
    {
        var connectionInfo = migrationType == MigrationType.Main
            ? _connectionStringFactory.MainDbConnection
            : _connectionStringFactory.LogDbConnection;

        _migrationController.Migrate(
            connectionInfo.ConnectionString,
            new MigrationContext(migrationType),
            connectionInfo.DatabaseType);

        return new Database(migrationType.ToString(), () =>
        {
            System.Data.IDbConnection conn = connectionInfo.DatabaseType == DatabaseType.SQLite
                ? new SqliteConnection(connectionInfo.ConnectionString)
                : new NpgsqlConnection(connectionInfo.ConnectionString);
            conn.Open();
            return conn;
        })
        {
            DatabaseType = connectionInfo.DatabaseType
        };
    }
}
