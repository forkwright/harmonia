// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Dapper;
using Microsoft.Data.Sqlite;
using Mouseion.Core.Datastore;
using Mouseion.Core.Progress;

namespace Mouseion.Core.Tests.Progress;

public class PlaybackSessionRepositoryTests : IDisposable
{
    private readonly SqliteConnection _connection;
    private readonly IDatabase _database;
    private readonly PlaybackSessionRepository _repository;

    public PlaybackSessionRepositoryTests()
    {
        var connectionString = $"DataSource=session_test_{Guid.NewGuid()};Mode=Memory;Cache=Shared";
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
            CREATE TABLE IF NOT EXISTS ""PlaybackSessions"" (
                ""Id"" INTEGER PRIMARY KEY AUTOINCREMENT,
                ""SessionId"" TEXT NOT NULL,
                ""MediaItemId"" INTEGER NOT NULL,
                ""UserId"" TEXT NOT NULL DEFAULT 'default',
                ""DeviceName"" TEXT NOT NULL DEFAULT '',
                ""DeviceType"" TEXT NOT NULL DEFAULT '',
                ""StartedAt"" TEXT NOT NULL,
                ""EndedAt"" TEXT,
                ""StartPositionMs"" INTEGER NOT NULL DEFAULT 0,
                ""EndPositionMs"" INTEGER,
                ""DurationMs"" INTEGER NOT NULL DEFAULT 0,
                ""IsActive"" INTEGER NOT NULL DEFAULT 1
            )");

        _repository = new PlaybackSessionRepository(_database);
    }

    [Fact]
    public async Task GetBySessionIdAsync_ReturnsNull_WhenNotFound()
    {
        var result = await _repository.GetBySessionIdAsync("nonexistent");
        Assert.Null(result);
    }

    [Fact]
    public async Task InsertAsync_CreatesSession()
    {
        var session = CreateSession(mediaItemId: 1, deviceName: "iPhone");
        var inserted = await _repository.InsertAsync(session);

        Assert.True(inserted.Id > 0);

        var found = await _repository.GetBySessionIdAsync(session.SessionId);
        Assert.NotNull(found);
        Assert.Equal("iPhone", found!.DeviceName);
        Assert.Equal(1, found.MediaItemId);
        Assert.True(found.IsActive);
    }

    [Fact]
    public async Task GetActiveSessionsAsync_ReturnsOnlyActive()
    {
        var active = CreateSession(mediaItemId: 1, isActive: true);
        var ended = CreateSession(mediaItemId: 2, isActive: false);

        await _repository.InsertAsync(active);
        await _repository.InsertAsync(ended);

        var results = await _repository.GetActiveSessionsAsync();

        Assert.Single(results);
        Assert.True(results[0].IsActive);
    }

    [Fact]
    public async Task GetRecentSessionsAsync_ReturnsAll_OrderedByStartDesc()
    {
        var older = CreateSession(mediaItemId: 1);
        older.StartedAt = DateTime.UtcNow.AddHours(-2);
        await _repository.InsertAsync(older);

        var newer = CreateSession(mediaItemId: 2);
        newer.StartedAt = DateTime.UtcNow;
        await _repository.InsertAsync(newer);

        var results = await _repository.GetRecentSessionsAsync();

        Assert.Equal(2, results.Count);
        Assert.Equal(2, results[0].MediaItemId);
        Assert.Equal(1, results[1].MediaItemId);
    }

    [Fact]
    public async Task GetRecentSessionsAsync_RespectsLimit()
    {
        for (int i = 0; i < 5; i++)
        {
            await _repository.InsertAsync(CreateSession(mediaItemId: 10 + i));
        }

        var results = await _repository.GetRecentSessionsAsync(limit: 3);
        Assert.Equal(3, results.Count);
    }

    [Fact]
    public async Task GetByMediaItemIdAsync_FiltersByMediaAndUser()
    {
        await _repository.InsertAsync(CreateSession(mediaItemId: 20, userId: "user-a"));
        await _repository.InsertAsync(CreateSession(mediaItemId: 20, userId: "user-b"));
        await _repository.InsertAsync(CreateSession(mediaItemId: 21, userId: "user-a"));

        var results = await _repository.GetByMediaItemIdAsync(20, "user-a");

        Assert.Single(results);
        Assert.Equal(20, results[0].MediaItemId);
        Assert.Equal("user-a", results[0].UserId);
    }

    [Fact]
    public async Task EndSessionAsync_MarksSessionInactive()
    {
        var session = CreateSession(mediaItemId: 1);
        await _repository.InsertAsync(session);

        await _repository.EndSessionAsync(session.SessionId, 150000);

        var ended = await _repository.GetBySessionIdAsync(session.SessionId);
        Assert.NotNull(ended);
        Assert.False(ended!.IsActive);
        Assert.Equal(150000, ended.EndPositionMs);
        Assert.NotNull(ended.EndedAt);
        Assert.True(ended.DurationMs > 0);
    }

    [Fact]
    public async Task DeleteAsync_RemovesSession()
    {
        var session = CreateSession(mediaItemId: 1);
        var inserted = await _repository.InsertAsync(session);

        await _repository.DeleteAsync(inserted.Id);

        var found = await _repository.GetBySessionIdAsync(session.SessionId);
        Assert.Null(found);
    }

    [Fact]
    public async Task GetActiveSessionsAsync_FiltersByUserId()
    {
        await _repository.InsertAsync(CreateSession(mediaItemId: 1, userId: "user-a"));
        await _repository.InsertAsync(CreateSession(mediaItemId: 2, userId: "user-b"));

        var resultsA = await _repository.GetActiveSessionsAsync("user-a");
        var resultsB = await _repository.GetActiveSessionsAsync("user-b");

        Assert.Single(resultsA);
        Assert.Single(resultsB);
        Assert.Equal("user-a", resultsA[0].UserId);
    }

    private static PlaybackSession CreateSession(
        int mediaItemId = 1,
        string userId = "default",
        string deviceName = "Test Device",
        string deviceType = "desktop",
        bool isActive = true)
    {
        return new PlaybackSession
        {
            SessionId = Guid.NewGuid().ToString(),
            MediaItemId = mediaItemId,
            UserId = userId,
            DeviceName = deviceName,
            DeviceType = deviceType,
            StartedAt = DateTime.UtcNow,
            StartPositionMs = 0,
            IsActive = isActive
        };
    }

    public void Dispose()
    {
        _connection?.Close();
        _connection?.Dispose();
    }
}
