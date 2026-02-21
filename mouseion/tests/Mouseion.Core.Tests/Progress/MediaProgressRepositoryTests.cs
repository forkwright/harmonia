// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Microsoft.Data.Sqlite;
using Mouseion.Core.Datastore;
using Mouseion.Core.Progress;

namespace Mouseion.Core.Tests.Progress;

public class MediaProgressRepositoryTests : IDisposable
{
    private readonly SqliteConnection _connection;
    private readonly IDatabase _database;
    private readonly MediaProgressRepository _repository;

    public MediaProgressRepositoryTests()
    {
        var connectionString = $"DataSource=progress_test_{Guid.NewGuid()};Mode=Memory;Cache=Shared";
        _connection = new SqliteConnection(connectionString);
        _connection.Open();

        _database = new Database("test", () =>
        {
            var conn = new SqliteConnection(connectionString);
            conn.Open();
            return conn;
        })
        {
            DatabaseType = DatabaseType.SQLite
        };

        using var conn = _database.OpenConnection();
        conn.Execute(@"
            CREATE TABLE IF NOT EXISTS ""MediaProgress"" (
                ""Id"" INTEGER PRIMARY KEY AUTOINCREMENT,
                ""MediaItemId"" INTEGER NOT NULL,
                ""UserId"" TEXT NOT NULL DEFAULT 'default',
                ""PositionMs"" INTEGER NOT NULL DEFAULT 0,
                ""TotalDurationMs"" INTEGER NOT NULL DEFAULT 0,
                ""PercentComplete"" REAL NOT NULL DEFAULT 0,
                ""LastPlayedAt"" TEXT NOT NULL,
                ""IsComplete"" INTEGER NOT NULL DEFAULT 0,
                ""CreatedAt"" TEXT NOT NULL,
                ""UpdatedAt"" TEXT NOT NULL
            )");

        _repository = new MediaProgressRepository(_database);
    }

    [Fact]
    public async Task GetByMediaItemIdAsync_ReturnsNull_WhenNotFound()
    {
        var result = await _repository.GetByMediaItemIdAsync(999);
        Assert.Null(result);
    }

    [Fact]
    public async Task UpsertAsync_InsertsNew_WhenNoExisting()
    {
        var progress = CreateProgress(mediaItemId: 1, positionMs: 60000, totalDurationMs: 180000);
        await _repository.UpsertAsync(progress);

        var result = await _repository.GetByMediaItemIdAsync(1);
        Assert.NotNull(result);
        Assert.Equal(60000, result!.PositionMs);
        Assert.Equal(180000, result.TotalDurationMs);
    }

    [Fact]
    public async Task UpsertAsync_UpdatesExisting_WhenAlreadyPresent()
    {
        await _repository.UpsertAsync(CreateProgress(mediaItemId: 2, positionMs: 10000));

        await _repository.UpsertAsync(CreateProgress(mediaItemId: 2, positionMs: 50000));

        var result = await _repository.GetByMediaItemIdAsync(2);
        Assert.NotNull(result);
        Assert.Equal(50000, result!.PositionMs);
    }

    [Fact]
    public async Task GetByMediaItemIdAsync_FiltersByUserId()
    {
        await _repository.UpsertAsync(CreateProgress(mediaItemId: 10, userId: "user-a", positionMs: 1000));
        await _repository.UpsertAsync(CreateProgress(mediaItemId: 10, userId: "user-b", positionMs: 2000));

        var resultA = await _repository.GetByMediaItemIdAsync(10, "user-a");
        var resultB = await _repository.GetByMediaItemIdAsync(10, "user-b");

        Assert.Equal(1000, resultA!.PositionMs);
        Assert.Equal(2000, resultB!.PositionMs);
    }

    [Fact]
    public async Task GetInProgressAsync_ReturnsOnlyIncomplete()
    {
        await _repository.UpsertAsync(CreateProgress(mediaItemId: 20, isComplete: false));
        await _repository.UpsertAsync(CreateProgress(mediaItemId: 21, isComplete: true));

        var results = await _repository.GetInProgressAsync();

        Assert.Single(results);
        Assert.Equal(20, results[0].MediaItemId);
    }

    [Fact]
    public async Task GetInProgressAsync_RespectsLimit()
    {
        for (int i = 0; i < 5; i++)
        {
            await _repository.UpsertAsync(CreateProgress(mediaItemId: 30 + i));
        }

        var results = await _repository.GetInProgressAsync(limit: 3);
        Assert.Equal(3, results.Count);
    }

    [Fact]
    public async Task GetInProgressAsync_OrdersByLastPlayedAtDescending()
    {
        var older = CreateProgress(mediaItemId: 40);
        older.LastPlayedAt = DateTime.UtcNow.AddHours(-2);
        await _repository.UpsertAsync(older);

        var newer = CreateProgress(mediaItemId: 41);
        newer.LastPlayedAt = DateTime.UtcNow;
        await _repository.UpsertAsync(newer);

        var results = await _repository.GetInProgressAsync();

        Assert.Equal(41, results[0].MediaItemId);
        Assert.Equal(40, results[1].MediaItemId);
    }

    [Fact]
    public async Task GetRecentlyPlayedAsync_ReturnsAllIncludingComplete()
    {
        await _repository.UpsertAsync(CreateProgress(mediaItemId: 50, isComplete: false));
        await _repository.UpsertAsync(CreateProgress(mediaItemId: 51, isComplete: true));

        var results = await _repository.GetRecentlyPlayedAsync();
        Assert.Equal(2, results.Count);
    }

    [Fact]
    public async Task DeleteByMediaItemIdAsync_RemovesProgress()
    {
        await _repository.UpsertAsync(CreateProgress(mediaItemId: 60));
        Assert.NotNull(await _repository.GetByMediaItemIdAsync(60));

        await _repository.DeleteByMediaItemIdAsync(60);
        Assert.Null(await _repository.GetByMediaItemIdAsync(60));
    }

    [Fact]
    public async Task DeleteByMediaItemIdAsync_OnlyDeletesMatchingUser()
    {
        await _repository.UpsertAsync(CreateProgress(mediaItemId: 70, userId: "user-a"));
        await _repository.UpsertAsync(CreateProgress(mediaItemId: 70, userId: "user-b"));

        await _repository.DeleteByMediaItemIdAsync(70, "user-a");

        Assert.Null(await _repository.GetByMediaItemIdAsync(70, "user-a"));
        Assert.NotNull(await _repository.GetByMediaItemIdAsync(70, "user-b"));
    }

    private static MediaProgress CreateProgress(
        int mediaItemId = 1,
        string userId = "default",
        long positionMs = 30000,
        long totalDurationMs = 180000,
        bool isComplete = false)
    {
        return new MediaProgress
        {
            MediaItemId = mediaItemId,
            UserId = userId,
            PositionMs = positionMs,
            TotalDurationMs = totalDurationMs,
            PercentComplete = totalDurationMs > 0
                ? Math.Round((decimal)positionMs / totalDurationMs * 100, 2)
                : 0,
            LastPlayedAt = DateTime.UtcNow,
            IsComplete = isComplete,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow
        };
    }

    public void Dispose()
    {
        _connection?.Close();
        _connection?.Dispose();
    }
}
