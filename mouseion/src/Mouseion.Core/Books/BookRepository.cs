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

namespace Mouseion.Core.Books;

public interface IBookRepository : IBasicRepository<Book>
{
    Task<Book?> FindByTitleAsync(string title, int year, CancellationToken ct = default);
    Task<List<Book>> GetByAuthorIdAsync(int authorId, CancellationToken ct = default);
    Task<List<Book>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default);
    Task<List<Book>> GetMonitoredAsync(CancellationToken ct = default);
    Task<bool> BookExistsAsync(int authorId, string title, int year, CancellationToken ct = default);

    Book? FindByTitle(string title, int year);
    List<Book> GetByAuthorId(int authorId);
    List<Book> GetBySeriesId(int seriesId);
    List<Book> GetMonitored();
    bool BookExists(int authorId, string title, int year);
}

public class BookRepository : BasicRepository<Book>, IBookRepository
{
    public BookRepository(IDatabase database)
        : base(database, "MediaItems")
    {
    }

    public override async Task<IEnumerable<Book>> AllAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryAsync<Book>($"SELECT * FROM \"MediaItems\" WHERE \"MediaType\" = {(int)MediaType.Book}").ConfigureAwait(false);
    }

    public override IEnumerable<Book> All()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Book>($"SELECT * FROM \"MediaItems\" WHERE \"MediaType\" = {(int)MediaType.Book}");
    }

    public override async Task<IEnumerable<Book>> GetPageAsync(int page, int pageSize, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var offset = (page - 1) * pageSize;
        return await conn.QueryAsync<Book>(
            $"SELECT * FROM \"MediaItems\" WHERE \"MediaType\" = {(int)MediaType.Book} ORDER BY \"Id\" DESC LIMIT @PageSize OFFSET @Offset",
            new { PageSize = pageSize, Offset = offset }).ConfigureAwait(false);
    }

    public override async Task<Book?> FindAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Book>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Id\" = @Id AND \"MediaType\" = {(int)MediaType.Book}",
            new { Id = id }).ConfigureAwait(false);
    }

    public override Book? Find(int id)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Book>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Id\" = @Id AND \"MediaType\" = {(int)MediaType.Book}",
            new { Id = id });
    }

    public async Task<Book?> FindByTitleAsync(string title, int year, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<Book>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Title\" = @Title AND \"Year\" = @Year AND \"MediaType\" = {(int)MediaType.Book}",
            new { Title = title, Year = year }).ConfigureAwait(false);
    }

    public Book? FindByTitle(string title, int year)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<Book>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Title\" = @Title AND \"Year\" = @Year AND \"MediaType\" = {(int)MediaType.Book}",
            new { Title = title, Year = year });
    }

    public async Task<List<Book>> GetByAuthorIdAsync(int authorId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Book>(
            $"SELECT * FROM \"MediaItems\" WHERE \"AuthorId\" = @AuthorId AND \"MediaType\" = {(int)MediaType.Book}",
            new { AuthorId = authorId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Book> GetByAuthorId(int authorId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Book>(
            $"SELECT * FROM \"MediaItems\" WHERE \"AuthorId\" = @AuthorId AND \"MediaType\" = {(int)MediaType.Book}",
            new { AuthorId = authorId }).ToList();
    }

    public async Task<List<Book>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Book>(
            $"SELECT * FROM \"MediaItems\" WHERE \"BookSeriesId\" = @SeriesId AND \"MediaType\" = {(int)MediaType.Book}",
            new { SeriesId = seriesId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Book> GetBySeriesId(int seriesId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Book>(
            $"SELECT * FROM \"MediaItems\" WHERE \"BookSeriesId\" = @SeriesId AND \"MediaType\" = {(int)MediaType.Book}",
            new { SeriesId = seriesId }).ToList();
    }

    public async Task<List<Book>> GetMonitoredAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<Book>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Monitored\" = @Monitored AND \"MediaType\" = {(int)MediaType.Book}",
            new { Monitored = true }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<Book> GetMonitored()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<Book>(
            $"SELECT * FROM \"MediaItems\" WHERE \"Monitored\" = @Monitored AND \"MediaType\" = {(int)MediaType.Book}",
            new { Monitored = true }).ToList();
    }

    public async Task<bool> BookExistsAsync(int authorId, string title, int year, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var count = await conn.QuerySingleAsync<int>(
            $"SELECT COUNT(*) FROM \"MediaItems\" WHERE \"AuthorId\" = @AuthorId AND \"Title\" = @Title AND \"Year\" = @Year AND \"MediaType\" = {(int)MediaType.Book}",
            new { AuthorId = authorId, Title = title, Year = year }).ConfigureAwait(false);
        return count > 0;
    }

    public bool BookExists(int authorId, string title, int year)
    {
        using var conn = _database.OpenConnection();
        var count = conn.QuerySingle<int>(
            $"SELECT COUNT(*) FROM \"MediaItems\" WHERE \"AuthorId\" = @AuthorId AND \"Title\" = @Title AND \"Year\" = @Year AND \"MediaType\" = {(int)MediaType.Book}",
            new { AuthorId = authorId, Title = title, Year = year });
        return count > 0;
    }
}
