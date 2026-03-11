// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.ImportLists.History;

public interface IImportSessionRepository : IBasicRepository<ImportSession>
{
    List<ImportSession> GetByListId(int listId);
    List<ImportSession> GetRecent(int count = 50);
    ImportSession? GetLatestForList(int listId);
}

public interface IImportSessionItemRepository : IBasicRepository<ImportSessionItem>
{
    List<ImportSessionItem> GetBySessionId(int sessionId);
    List<ImportSessionItem> GetConflicts(int sessionId);
}

public class ImportSessionRepository : BasicRepository<ImportSession>, IImportSessionRepository
{
    private new readonly IDatabase _database;

    public ImportSessionRepository(IDatabase database) : base(database)
    {
        _database = database;
    }

    public List<ImportSession> GetByListId(int listId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<ImportSession>(
            "SELECT * FROM \"ImportSessions\" WHERE \"ImportListId\" = @ListId ORDER BY \"StartedAt\" DESC",
            new { ListId = listId }
        ).ToList();
    }

    public List<ImportSession> GetRecent(int count = 50)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<ImportSession>(
            "SELECT * FROM \"ImportSessions\" ORDER BY \"StartedAt\" DESC LIMIT @Count",
            new { Count = count }
        ).ToList();
    }

    public ImportSession? GetLatestForList(int listId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<ImportSession>(
            "SELECT * FROM \"ImportSessions\" WHERE \"ImportListId\" = @ListId ORDER BY \"StartedAt\" DESC LIMIT 1",
            new { ListId = listId }
        );
    }
}

public class ImportSessionItemRepository : BasicRepository<ImportSessionItem>, IImportSessionItemRepository
{
    private new readonly IDatabase _database;

    public ImportSessionItemRepository(IDatabase database) : base(database)
    {
        _database = database;
    }

    public List<ImportSessionItem> GetBySessionId(int sessionId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<ImportSessionItem>(
            "SELECT * FROM \"ImportSessionItems\" WHERE \"SessionId\" = @SessionId ORDER BY \"ProcessedAt\"",
            new { SessionId = sessionId }
        ).ToList();
    }

    public List<ImportSessionItem> GetConflicts(int sessionId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<ImportSessionItem>(
            "SELECT * FROM \"ImportSessionItems\" WHERE \"SessionId\" = @SessionId AND \"Action\" = @Action ORDER BY \"ProcessedAt\"",
            new { SessionId = sessionId, Action = (int)ImportItemAction.Conflict }
        ).ToList();
    }
}
