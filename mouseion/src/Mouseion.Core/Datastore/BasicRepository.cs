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

    public virtual IEnumerable<TModel> All()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<TModel>($"SELECT * FROM \"{_table}\"");
    }

    public int Count()
    {
        using var conn = _database.OpenConnection();
        return conn.QuerySingle<int>($"SELECT COUNT(*) FROM \"{_table}\"");
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

    public TModel? Find(int id)
    {
        using var conn = _database.OpenConnection();
        return conn.QuerySingleOrDefault<TModel>(
            $"SELECT * FROM \"{_table}\" WHERE \"Id\" = @Id",
            new { Id = id });
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

    public TModel Update(TModel model)
    {
        using var conn = _database.OpenConnection();
        var sql = BuildUpdateSql(model);

        RetryStrategy.Execute(() => conn.Execute(sql, model));
        return model;
    }

    public void Delete(int id)
    {
        using var conn = _database.OpenConnection();
        RetryStrategy.Execute(() => conn.Execute(
            $"DELETE FROM \"{_table}\" WHERE \"Id\" = @Id",
            new { Id = id }));
    }

    public IEnumerable<TModel> Get(IEnumerable<int> ids)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<TModel>(
            $"SELECT * FROM \"{_table}\" WHERE \"Id\" IN @Ids",
            new { Ids = ids });
    }

    public void InsertMany(IList<TModel> models)
    {
        foreach (var model in models)
        {
            Insert(model);
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
        var properties = typeof(TModel).GetProperties()
            .Where(p => p.Name != "Id")
            .ToList();

        var columns = string.Join(", ", properties.Select(p => $"\"{p.Name}\""));
        var values = string.Join(", ", properties.Select(p => $"@{p.Name}"));

        return $"INSERT INTO \"{_table}\" ({columns}) VALUES ({values}); SELECT last_insert_rowid()";
    }

    private string BuildInsertSqlPostgreSQL(TModel model)
    {
        var properties = typeof(TModel).GetProperties()
            .Where(p => p.Name != "Id")
            .ToList();

        var columns = string.Join(", ", properties.Select(p => $"\"{p.Name}\""));
        var values = string.Join(", ", properties.Select(p => $"@{p.Name}"));

        return $"INSERT INTO \"{_table}\" ({columns}) VALUES ({values}) RETURNING \"Id\"";
    }

    private string BuildUpdateSql(TModel model)
    {
        var properties = typeof(TModel).GetProperties()
            .Where(p => p.Name != "Id")
            .ToList();

        var setClause = string.Join(", ", properties.Select(p => $"\"{p.Name}\" = @{p.Name}"));

        return $"UPDATE \"{_table}\" SET {setClause} WHERE \"Id\" = @Id";
    }
}
