// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Progress;
using Mouseion.Core.Tests.Repositories;

namespace Mouseion.Core.Tests.Progress;

public class PlaybackSessionRepositoryTests : RepositoryTestBase
{
    private readonly PlaybackSessionRepository _repo;

    public PlaybackSessionRepositoryTests()
    {
        _repo = new PlaybackSessionRepository(Database);
    }

    [Fact]
    public async Task InsertAndGetBySessionId_RoundTrips()
    {
        var session = new PlaybackSession
        {
            SessionId = "test-session-1",
            MediaItemId = 1,
            UserId = "default",
            UserIdInt = 1,
            DeviceName = "Desktop",
            DeviceType = "PC",
            StartedAt = DateTime.UtcNow,
            StartPositionMs = 0,
            IsActive = true
        };

        await _repo.InsertAsync(session);

        var result = await _repo.GetBySessionIdAsync("test-session-1");
        Assert.NotNull(result);
        Assert.Equal(1, result.MediaItemId);
        Assert.Equal("Desktop", result.DeviceName);
        Assert.True(result.IsActive);
    }

    [Fact]
    public async Task GetActiveSessionsAsync_OnlyReturnsActive()
    {
        await _repo.InsertAsync(new PlaybackSession
        {
            SessionId = "active-1",
            MediaItemId = 1,
            UserId = "default",
            StartedAt = DateTime.UtcNow,
            IsActive = true
        });

        await _repo.InsertAsync(new PlaybackSession
        {
            SessionId = "ended-1",
            MediaItemId = 2,
            UserId = "default",
            StartedAt = DateTime.UtcNow.AddMinutes(-30),
            EndedAt = DateTime.UtcNow,
            IsActive = false
        });

        var active = await _repo.GetActiveSessionsAsync("default");
        Assert.Single(active);
        Assert.Equal("active-1", active[0].SessionId);
    }

    [Fact]
    public async Task EndSessionAsync_SetsEndedAtAndIsActive()
    {
        await _repo.InsertAsync(new PlaybackSession
        {
            SessionId = "end-test-1",
            MediaItemId = 1,
            UserId = "default",
            StartedAt = DateTime.UtcNow,
            IsActive = true
        });

        await _repo.EndSessionAsync("end-test-1", 45000);

        var result = await _repo.GetBySessionIdAsync("end-test-1");
        Assert.NotNull(result);
        Assert.False(result.IsActive);
        Assert.NotNull(result.EndedAt);
        Assert.Equal(45000, result.EndPositionMs);
    }

    [Fact]
    public async Task GetByMediaItemIdAsync_FiltersByUserAndMediaItem()
    {
        await _repo.InsertAsync(new PlaybackSession
        {
            SessionId = "user-a-1",
            MediaItemId = 5,
            UserId = "user-a",
            StartedAt = DateTime.UtcNow,
            IsActive = false
        });

        await _repo.InsertAsync(new PlaybackSession
        {
            SessionId = "user-b-1",
            MediaItemId = 5,
            UserId = "user-b",
            StartedAt = DateTime.UtcNow,
            IsActive = false
        });

        var userA = await _repo.GetByMediaItemIdAsync(5, "user-a");
        Assert.Single(userA);
        Assert.Equal("user-a-1", userA[0].SessionId);
    }

    [Fact]
    public async Task GetRecentSessionsAsync_RespectsLimit()
    {
        for (int i = 0; i < 5; i++)
        {
            await _repo.InsertAsync(new PlaybackSession
            {
                SessionId = $"limit-{i}",
                MediaItemId = i + 1,
                UserId = "limit-user",
                StartedAt = DateTime.UtcNow.AddMinutes(-i),
                IsActive = false
            });
        }

        var result = await _repo.GetRecentSessionsAsync("limit-user", limit: 3);
        Assert.Equal(3, result.Count);
    }

    [Fact]
    public async Task DeleteAsync_RemovesSession()
    {
        var session = await _repo.InsertAsync(new PlaybackSession
        {
            SessionId = "delete-me",
            MediaItemId = 1,
            UserId = "default",
            StartedAt = DateTime.UtcNow,
            IsActive = false
        });

        await _repo.DeleteAsync(session.Id);

        var result = await _repo.GetBySessionIdAsync("delete-me");
        Assert.Null(result);
    }
}
