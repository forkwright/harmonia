// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Progress;
using Mouseion.Core.Tests.Repositories;

namespace Mouseion.Core.Tests.Progress;

public class MediaProgressRepositoryTests : RepositoryTestBase
{
    private readonly MediaProgressRepository _repo;

    public MediaProgressRepositoryTests()
    {
        _repo = new MediaProgressRepository(Database);
    }

    [Fact]
    public async Task GetByMediaItemIdAsync_NonExistent_ReturnsNull()
    {
        var result = await _repo.GetByMediaItemIdAsync(999);
        Assert.Null(result);
    }

    [Fact]
    public async Task UpsertAsync_NewRecord_Inserts()
    {
        var progress = new MediaProgress
        {
            MediaItemId = 1,
            UserId = "1",
            UserIdInt = 1,
            PositionMs = 5000,
            TotalDurationMs = 100000,
            PercentComplete = 5.0m,
            LastPlayedAt = DateTime.UtcNow,
            IsComplete = false
        };

        await _repo.UpsertAsync(progress);

        var result = await _repo.GetByMediaItemIdAsync(1);
        Assert.NotNull(result);
        Assert.Equal(5000, result.PositionMs);
        Assert.Equal(100000, result.TotalDurationMs);
    }

    [Fact]
    public async Task UpsertAsync_ExistingRecord_Updates()
    {
        // Insert
        var progress = new MediaProgress
        {
            MediaItemId = 2,
            UserId = "default",
            PositionMs = 1000,
            TotalDurationMs = 50000,
            PercentComplete = 2.0m,
            LastPlayedAt = DateTime.UtcNow
        };
        await _repo.UpsertAsync(progress);

        // Update
        progress.PositionMs = 25000;
        progress.PercentComplete = 50.0m;
        await _repo.UpsertAsync(progress);

        var result = await _repo.GetByMediaItemIdAsync(2);
        Assert.NotNull(result);
        Assert.Equal(25000, result.PositionMs);
        Assert.Equal(50.0m, result.PercentComplete);
    }

    [Fact]
    public async Task GetInProgressAsync_ExcludesCompleted()
    {
        await _repo.UpsertAsync(new MediaProgress
        {
            MediaItemId = 10,
            UserId = "default",
            PositionMs = 5000,
            TotalDurationMs = 100000,
            IsComplete = false,
            LastPlayedAt = DateTime.UtcNow
        });

        await _repo.UpsertAsync(new MediaProgress
        {
            MediaItemId = 11,
            UserId = "default",
            PositionMs = 100000,
            TotalDurationMs = 100000,
            IsComplete = true,
            LastPlayedAt = DateTime.UtcNow
        });

        var inProgress = await _repo.GetInProgressAsync("default");
        Assert.Single(inProgress);
        Assert.Equal(10, inProgress[0].MediaItemId);
    }

    [Fact]
    public async Task GetInProgressAsync_UserIsolation()
    {
        await _repo.UpsertAsync(new MediaProgress
        {
            MediaItemId = 20,
            UserId = "user-a",
            PositionMs = 5000,
            TotalDurationMs = 100000,
            IsComplete = false,
            LastPlayedAt = DateTime.UtcNow
        });

        await _repo.UpsertAsync(new MediaProgress
        {
            MediaItemId = 21,
            UserId = "user-b",
            PositionMs = 10000,
            TotalDurationMs = 100000,
            IsComplete = false,
            LastPlayedAt = DateTime.UtcNow
        });

        var userA = await _repo.GetInProgressAsync("user-a");
        Assert.Single(userA);
        Assert.Equal(20, userA[0].MediaItemId);

        var userB = await _repo.GetInProgressAsync("user-b");
        Assert.Single(userB);
        Assert.Equal(21, userB[0].MediaItemId);
    }

    [Fact]
    public async Task DeleteByMediaItemIdAsync_RemovesRecord()
    {
        await _repo.UpsertAsync(new MediaProgress
        {
            MediaItemId = 30,
            UserId = "default",
            PositionMs = 5000,
            TotalDurationMs = 100000,
            LastPlayedAt = DateTime.UtcNow
        });

        await _repo.DeleteByMediaItemIdAsync(30);

        var result = await _repo.GetByMediaItemIdAsync(30);
        Assert.Null(result);
    }

    [Fact]
    public async Task GetRecentlyPlayedAsync_OrdersByLastPlayedDesc()
    {
        var now = DateTime.UtcNow;

        await _repo.UpsertAsync(new MediaProgress
        {
            MediaItemId = 40,
            UserId = "default",
            PositionMs = 1000,
            TotalDurationMs = 50000,
            LastPlayedAt = now.AddHours(-2)
        });

        await _repo.UpsertAsync(new MediaProgress
        {
            MediaItemId = 41,
            UserId = "default",
            PositionMs = 2000,
            TotalDurationMs = 50000,
            LastPlayedAt = now
        });

        var recent = await _repo.GetRecentlyPlayedAsync("default");
        Assert.Equal(2, recent.Count);
        Assert.Equal(41, recent[0].MediaItemId); // Most recent first
        Assert.Equal(40, recent[1].MediaItemId);
    }

    [Fact]
    public async Task GetInProgressAsync_RespectsLimit()
    {
        for (int i = 50; i < 55; i++)
        {
            await _repo.UpsertAsync(new MediaProgress
            {
                MediaItemId = i,
                UserId = "limit-test",
                PositionMs = 1000,
                TotalDurationMs = 50000,
                IsComplete = false,
                LastPlayedAt = DateTime.UtcNow
            });
        }

        var result = await _repo.GetInProgressAsync("limit-test", limit: 3);
        Assert.Equal(3, result.Count);
    }
}
