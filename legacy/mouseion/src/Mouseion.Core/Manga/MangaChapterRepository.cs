// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Mouseion.Core.Datastore;

namespace Mouseion.Core.Manga;

public interface IMangaChapterRepository : IBasicRepository<MangaChapter>
{
    Task<List<MangaChapter>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default);
    Task<MangaChapter?> FindByMangaDexChapterIdAsync(string chapterId, CancellationToken ct = default);
    Task<MangaChapter?> FindByChapterNumberAsync(int seriesId, decimal chapterNumber, CancellationToken ct = default);
    Task<List<MangaChapter>> GetUnreadAsync(CancellationToken ct = default);
    Task<List<MangaChapter>> GetUnreadBySeriesAsync(int seriesId, CancellationToken ct = default);
    Task<int> GetUnreadCountAsync(CancellationToken ct = default);
    Task<int> GetUnreadCountBySeriesAsync(int seriesId, CancellationToken ct = default);
    Task MarkReadAsync(int id, CancellationToken ct = default);
    Task MarkUnreadAsync(int id, CancellationToken ct = default);
    Task MarkAllReadBySeriesAsync(int seriesId, CancellationToken ct = default);

    List<MangaChapter> GetBySeriesId(int seriesId);
    MangaChapter? FindByMangaDexChapterId(string chapterId);
    List<MangaChapter> GetUnread();
    int GetUnreadCount();
}

public class MangaChapterRepository : BasicRepository<MangaChapter>, IMangaChapterRepository
{
    public MangaChapterRepository(IDatabase database)
        : base(database, "MangaChapters")
    {
    }

    public async Task<List<MangaChapter>> GetBySeriesIdAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<MangaChapter>(
            "SELECT * FROM \"MangaChapters\" WHERE \"MangaSeriesId\" = @SeriesId ORDER BY \"ChapterNumber\" DESC",
            new { SeriesId = seriesId }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<MangaChapter> GetBySeriesId(int seriesId)
    {
        using var conn = _database.OpenConnection();
        return conn.Query<MangaChapter>(
            "SELECT * FROM \"MangaChapters\" WHERE \"MangaSeriesId\" = @SeriesId ORDER BY \"ChapterNumber\" DESC",
            new { SeriesId = seriesId }).ToList();
    }

    public async Task<MangaChapter?> FindByMangaDexChapterIdAsync(string chapterId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<MangaChapter>(
            "SELECT * FROM \"MangaChapters\" WHERE \"MangaDexChapterId\" = @ChapterId",
            new { ChapterId = chapterId }).ConfigureAwait(false);
    }

    public MangaChapter? FindByMangaDexChapterId(string chapterId)
    {
        using var conn = _database.OpenConnection();
        return conn.QueryFirstOrDefault<MangaChapter>(
            "SELECT * FROM \"MangaChapters\" WHERE \"MangaDexChapterId\" = @ChapterId",
            new { ChapterId = chapterId });
    }

    public async Task<MangaChapter?> FindByChapterNumberAsync(int seriesId, decimal chapterNumber, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.QueryFirstOrDefaultAsync<MangaChapter>(
            "SELECT * FROM \"MangaChapters\" WHERE \"MangaSeriesId\" = @SeriesId AND \"ChapterNumber\" = @ChapterNumber",
            new { SeriesId = seriesId, ChapterNumber = chapterNumber }).ConfigureAwait(false);
    }

    public async Task<List<MangaChapter>> GetUnreadAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<MangaChapter>(
            "SELECT * FROM \"MangaChapters\" WHERE \"IsRead\" = 0 ORDER BY \"ChapterNumber\" DESC",
            new { }).ConfigureAwait(false);
        return result.ToList();
    }

    public List<MangaChapter> GetUnread()
    {
        using var conn = _database.OpenConnection();
        return conn.Query<MangaChapter>(
            "SELECT * FROM \"MangaChapters\" WHERE \"IsRead\" = 0 ORDER BY \"ChapterNumber\" DESC",
            new { }).ToList();
    }

    public async Task<List<MangaChapter>> GetUnreadBySeriesAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        var result = await conn.QueryAsync<MangaChapter>(
            "SELECT * FROM \"MangaChapters\" WHERE \"MangaSeriesId\" = @SeriesId AND \"IsRead\" = 0 ORDER BY \"ChapterNumber\" ASC",
            new { SeriesId = seriesId }).ConfigureAwait(false);
        return result.ToList();
    }

    public async Task<int> GetUnreadCountAsync(CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.ExecuteScalarAsync<int>(
            "SELECT COUNT(*) FROM \"MangaChapters\" WHERE \"IsRead\" = 0").ConfigureAwait(false);
    }

    public async Task<int> GetUnreadCountBySeriesAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        return await conn.ExecuteScalarAsync<int>(
            "SELECT COUNT(*) FROM \"MangaChapters\" WHERE \"IsRead\" = 0 AND \"MangaSeriesId\" = @SeriesId",
            new { SeriesId = seriesId }).ConfigureAwait(false);
    }

    public int GetUnreadCount()
    {
        using var conn = _database.OpenConnection();
        return conn.ExecuteScalar<int>(
            "SELECT COUNT(*) FROM \"MangaChapters\" WHERE \"IsRead\" = 0");
    }

    public async Task MarkReadAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "UPDATE \"MangaChapters\" SET \"IsRead\" = 1 WHERE \"Id\" = @Id",
            new { Id = id }).ConfigureAwait(false);
    }

    public async Task MarkUnreadAsync(int id, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "UPDATE \"MangaChapters\" SET \"IsRead\" = 0 WHERE \"Id\" = @Id",
            new { Id = id }).ConfigureAwait(false);
    }

    public async Task MarkAllReadBySeriesAsync(int seriesId, CancellationToken ct = default)
    {
        using var conn = _database.OpenConnection();
        await conn.ExecuteAsync(
            "UPDATE \"MangaChapters\" SET \"IsRead\" = 1 WHERE \"MangaSeriesId\" = @SeriesId",
            new { SeriesId = seriesId }).ConfigureAwait(false);
    }
}
