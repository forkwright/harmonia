// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Microsoft.Data.Sqlite;
using Polly;
using Polly.Retry;

namespace Mouseion.Core.Datastore;

// Generic repository with Polly retry for SQLite busy errors
public class BasicRepository<TModel> : IBasicRepository<TModel>
    where TModel : ModelBase, new()
{
    protected readonly IDatabase _database;
    protected readonly string _table;

    private static readonly ResiliencePipeline RetryStrategy =
        new ResiliencePipelineBuilder()
            .AddRetry(new RetryStrategyOptions
            {
                ShouldHandle = new PredicateBuilder()
                    .Handle<SqliteException>(ex => ex.SqliteErrorCode == 5), // SQLITE_BUSY
                Delay = TimeSpan.FromMilliseconds(100),
                MaxRetryAttempts = 3,
                BackoffType = DelayBackoffType.Exponential
            })
            .Build();

    public BasicRepository(IDatabase database)
    {
        _database = database;
        _table = typeof(TModel).Name + "s";
    }

    protected BasicRepository(IDatabase database, string tableName)
    {
        _database = database;
        _table = tableName;
    }

    public virtual async Task<IEnumerable<TModel>> AllAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryAsync<TModel>($"SELECT * FROM \"{_table}\"").ConfigureAwait(false);
    }

    public virtual IEnumerable<TModel> All()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<TModel>($"SELECT * FROM \"{_table}\"");
    }

    public async Task<int> CountAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QuerySingleAsync<int>($"SELECT COUNT(*) FROM \"{_table}\"").ConfigureAwait(false);
    }

    public int Count()
    {
        using var conn = _database.OpenConnection();
        return conn.QuerySingle<int>($"SELECT COUNT(*) FROM \"{_table}\"");
    }

    public virtual async Task<IEnumerable<TModel>> GetPageAsync(int page, int pageSize, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var offset = (page - 1) * pageSize;
        return await conn.QueryAsync<TModel>(
            $"SELECT * FROM \"{_table}\" ORDER BY \"Id\" DESC LIMIT @PageSize OFFSET @Offset",
            new { PageSize = pageSize, Offset = offset }).ConfigureAwait(false);
    }

    public async Task<TModel> GetAsync(int id, CancellationToken ct = default)
    {
        var model = await FindAsync(id, ct).ConfigureAwait(false);
        if (model == null)
        {
            throw new KeyNotFoundException($"{typeof(TModel).Name} with ID {id} not found");
        }

        return model;
    }

    public TModel Get(int id)
    {
        var model = Find(id);
        if (model == null)
        {
            throw new KeyNotFoundException($"{typeof(TModel).Name} with ID {id} not found");
        }

        return model;
    }

    public virtual async Task<TModel?> FindAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QuerySingleOrDefaultAsync<TModel>(
            $"SELECT * FROM \"{_table}\" WHERE \"Id\" = @Id",
            new { Id = id }).ConfigureAwait(false);
    }

    public virtual TModel? Find(int id)
    {
        using var conn = _database.OpenConnection();
        return conn.QuerySingleOrDefault<TModel>(
            $"SELECT * FROM \"{_table}\" WHERE \"Id\" = @Id",
            new { Id = id });
    }

    public async Task<TModel> InsertAsync(TModel model, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();

        var sql = _database.DatabaseType == DatabaseType.PostgreSQL
            ? BuildInsertSqlPostgreSQL(model)
            : BuildInsertSqlSQLite(model);

        var id = await RetryStrategy.ExecuteAsync(async _ =>
            await conn.QuerySingleAsync<int>(sql, model).ConfigureAwait(false), ct).ConfigureAwait(false);
        model.Id = id;
        return model;
    }

    public TModel Insert(TModel model)
    {
        using var conn = _database.OpenConnection();

        var sql = _database.DatabaseType == DatabaseType.PostgreSQL
            ? BuildInsertSqlPostgreSQL(model)
            : BuildInsertSqlSQLite(model);

        var id = RetryStrategy.Execute(() => conn.QuerySingle<int>(sql, model));
        model.Id = id;
        return model;
    }

    public async Task<TModel> UpdateAsync(TModel model, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var sql = BuildUpdateSql(model);

        await RetryStrategy.ExecuteAsync(async _ =>
            await conn.ExecuteAsync(sql, model).ConfigureAwait(false), ct).ConfigureAwait(false);
        return model;
    }

    public TModel Update(TModel model)
    {
        using var conn = _database.OpenConnection();
        var sql = BuildUpdateSql(model);

        RetryStrategy.Execute(() => conn.Execute(sql, model));
        return model;
    }

    public async Task DeleteAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await RetryStrategy.ExecuteAsync(async _ =>
            await conn.ExecuteAsync(
                $"DELETE FROM \"{_table}\" WHERE \"Id\" = @Id",
                new { Id = id }).ConfigureAwait(false), ct).ConfigureAwait(false);
    }

    public void Delete(int id)
    {
        using var conn = _database.OpenConnection();
        RetryStrategy.Execute(() => conn.Execute(
            $"DELETE FROM \"{_table}\" WHERE \"Id\" = @Id",
            new { Id = id }));
    }

    public async Task<IEnumerable<TModel>> GetAsync(IEnumerable<int> ids, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryAsync<TModel>(
            $"SELECT * FROM \"{_table}\" WHERE \"Id\" IN @Ids",
            new { Ids = ids }).ConfigureAwait(false);
    }

    public IEnumerable<TModel> Get(IEnumerable<int> ids)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<TModel>(
            $"SELECT * FROM \"{_table}\" WHERE \"Id\" IN @Ids",
            new { Ids = ids });
    }

    public async Task InsertManyAsync(IList<TModel> models, CancellationToken ct = default)
    {
        foreach (var model in models)
        {
            await InsertAsync(model, ct).ConfigureAwait(false);
        }
    }

    public void InsertMany(IList<TModel> models)
    {
        foreach (var model in models)
        {
            Insert(model);
        }
    }

    public async Task UpdateManyAsync(IList<TModel> models, CancellationToken ct = default)
    {
        foreach (var model in models)
        {
            await UpdateAsync(model, ct).ConfigureAwait(false);
        }
    }

    public void UpdateMany(IList<TModel> models)
    {
        foreach (var model in models)
        {
            Update(model);
        }
    }

    private string BuildInsertSqlSQLite(TModel model)
    {
        var properties = GetDatabaseProperties();

        var columns = string.Join(", ", properties.Select(p => $"\"{p.Name}\""));
        var values = string.Join(", ", properties.Select(p => $"@{p.Name}"));

        return $"INSERT INTO \"{_table}\" ({columns}) VALUES ({values}); SELECT last_insert_rowid()";
    }

    private string BuildInsertSqlPostgreSQL(TModel model)
    {
        var properties = GetDatabaseProperties();

        var columns = string.Join(", ", properties.Select(p => $"\"{p.Name}\""));
        var values = string.Join(", ", properties.Select(p => $"@{p.Name}"));

        return $"INSERT INTO \"{_table}\" ({columns}) VALUES ({values}) RETURNING \"Id\"";
    }

    private string BuildUpdateSql(TModel model)
    {
        var properties = GetDatabaseProperties();

        var setClause = string.Join(", ", properties.Select(p => $"\"{p.Name}\" = @{p.Name}"));

        return $"UPDATE \"{_table}\" SET {setClause} WHERE \"Id\" = @Id";
    }

    private static global::System.Reflection.PropertyInfo[] GetDatabaseProperties()
    {
        return typeof(TModel).GetProperties()
            .Where(p => p.Name != "Id")
            .Where(p => IsDatabaseType(p.PropertyType))
            .ToArray();
    }

    private static bool IsDatabaseType(Type type)
    {
        // Primitive types, strings, enums, nullables, DateTimes are database types
        if (type.IsPrimitive || type == typeof(string) || type.IsEnum || type == typeof(DateTime))
            return true;

        // Nullable<T>
        if (type.IsGenericType && type.GetGenericTypeDefinition() == typeof(Nullable<>))
        {
            var underlyingType = Nullable.GetUnderlyingType(type);
            return underlyingType != null && IsDatabaseType(underlyingType);
        }

        return false;
    }
}
