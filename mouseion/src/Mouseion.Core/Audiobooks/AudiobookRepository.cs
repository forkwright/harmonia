// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Audiobooks;

public interface IAudiobookRepository : IBasicRepository<Audiobook>
{
    Task<Audiobook?> FindByTitleAsync(string title, int year, CancellationToken ct = default);
    Task<List<Audiobook>> GetByAuthorIdAsync(int authorId, CancellationToken ct = default);
    Task<List<Audiobook>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default);
    Task<List<Audiobook>> GetMonitoredAsync(CancellationToken ct = default);
    Task<bool> AudiobookExistsAsync(int authorId, string title, int year, CancellationToken ct = default);

    Audiobook? FindByTitle(string title, int year);
    List<Audiobook> GetByAuthorId(int authorId);
    List<Audiobook> GetBySeriesId(int seriesId);
    List<Audiobook> GetMonitored();
    bool AudiobookExists(int authorId, string title, int year);
}

public class AudiobookRepository : BasicRepository<Audiobook>, IAudiobookRepository
{
    public AudiobookRepository(IDatabase database)
        : base(database, "MediaItems")
    {
    }

    public override async Task<IEnumerable<Audiobook>> AllAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryAsync<Audiobook>($"SELECT * FROM \"MediaItems\" WHERE \"MediaType\" = {(int)MediaType.Audiobook}").ConfigureAwait(false);
    }

    public override IEnumerable<Audiobook> All()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Audiobook>($"SELECT * FROM \"MediaItems\" WHERE \"MediaType\" = {(int)MediaType.Audiobook}");
    }

    public override async Task<IEnumerable<Audiobook>> GetPageAsync(int page, int pageSize, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var offset = (page - 1) * pageSize;
        return await conn.QueryAsync<Audiobook>(
            $"SELECT * FROM \"MediaItems\" WHERE \"MediaType\" = {(int)MediaType.Audiobook} ORDER BY \"Id\" DESC LIMIT @PageSize OFFSET @Offset",
            new { PageSize = pageSize, Offset = offset }).ConfigureAwait(false);
    }

    public override async Task<Audiobook?> FindAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Audiobook>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Id\" = @Id AND \"MediaType\" = {(int)MediaType.Audiobook}",
            new { Id = id }).ConfigureAwait(false);
    }

    public override Audiobook? Find(int id)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Audiobook>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Id\" = @Id AND \"MediaType\" = {(int)MediaType.Audiobook}",
            new { Id = id });
    }

    public async Task<Audiobook?> FindByTitleAsync(string title, int year, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Audiobook>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Title\" = @Title AND \"Year\" = @Year AND \"MediaType\" = {(int)MediaType.Audiobook}",
            new { Title = title, Year = year }).ConfigureAwait(false);
    }

    public Audiobook? FindByTitle(string title, int year)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Audiobook>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Title\" = @Title AND \"Year\" = @Year AND \"MediaType\" = {(int)MediaType.Audiobook}",
            new { Title = title, Year = year });
    }

    public async Task<List<Audiobook>> GetByAuthorIdAsync(int authorId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Audiobook>(
            $"SELECT * FROM \"MediaItems\" WHERE \"AuthorId\" = @AuthorId AND \"MediaType\" = {(int)MediaType.Audiobook}",
            new { AuthorId = authorId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Audiobook> GetByAuthorId(int authorId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Audiobook>(
            $"SELECT * FROM \"MediaItems\" WHERE \"AuthorId\" = @AuthorId AND \"MediaType\" = {(int)MediaType.Audiobook}",
            new { AuthorId = authorId }).ToList();
    }

    public async Task<List<Audiobook>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Audiobook>(
            $"SELECT * FROM \"MediaItems\" WHERE \"BookSeriesId\" = @SeriesId AND \"MediaType\" = {(int)MediaType.Audiobook}",
            new { SeriesId = seriesId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Audiobook> GetBySeriesId(int seriesId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Audiobook>(
            $"SELECT * FROM \"MediaItems\" WHERE \"BookSeriesId\" = @SeriesId AND \"MediaType\" = {(int)MediaType.Audiobook}",
            new { SeriesId = seriesId }).ToList();
    }

    public async Task<List<Audiobook>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Audiobook>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Monitored\" = @Monitored AND \"MediaType\" = {(int)MediaType.Audiobook}",
            new { Monitored = true }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Audiobook> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Audiobook>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Monitored\" = @Monitored AND \"MediaType\" = {(int)MediaType.Audiobook}",
            new { Monitored = true }).ToList();
    }

    public async Task<bool> AudiobookExistsAsync(int authorId, string title, int year, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var count = await conn.QuerySingleAsync<int>(
            $"SELECT COUNT(*) FROM \"MediaItems\" WHERE \"AuthorId\" = @AuthorId AND \"Title\" = @Title AND \"Year\" = @Year AND \"MediaType\" = {(int)MediaType.Audiobook}",
            new { AuthorId = authorId, Title = title, Year = year }).ConfigureAwait(false);
        return count > 0;
    }

    public bool AudiobookExists(int authorId, string title, int year)
    {
        using var conn = _database.OpenConnection();
        var count = conn.QuerySingle<int>(
            $"SELECT COUNT(*) FROM \"MediaItems\" WHERE \"AuthorId\" = @AuthorId AND \"Title\" = @Title AND \"Year\" = @Year AND \"MediaType\" = {(int)MediaType.Audiobook}",
            new { AuthorId = authorId, Title = title, Year = year });
        return count > 0;
    }
}
