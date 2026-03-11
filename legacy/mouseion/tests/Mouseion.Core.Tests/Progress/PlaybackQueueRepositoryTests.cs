// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Progress;
using Mouseion.Core.Tests.Repositories;

namespace Mouseion.Core.Tests.Progress;

public class PlaybackQueueRepositoryTests : RepositoryTestBase
{
    private readonly PlaybackQueueRepository _repo;

    public PlaybackQueueRepositoryTests()
    {
        _repo = new PlaybackQueueRepository(Database);
    }

    [Fact]
    public async Task GetByUserAndDevice_NonExistent_ReturnsNull()
    {
        var result = await _repo.GetByUserAndDeviceAsync(1, "nonexistent");
        Assert.Null(result);
    }

    [Fact]
    public async Task UpsertAsync_NewQueue_Inserts()
    {
        var queue = new PlaybackQueue
        {
            UserId = 1,
            DeviceName = "Phone",
            QueueData = "[{\"mediaItemId\":1}]",
            CurrentIndex = 0,
            ShuffleEnabled = false,
            RepeatMode = "none"
        };

        await _repo.UpsertAsync(queue);

        var result = await _repo.GetByUserAndDeviceAsync(1, "Phone");
        Assert.NotNull(result);
        Assert.Equal("[{\"mediaItemId\":1}]", result.QueueData);
    }

    [Fact]
    public async Task UpsertAsync_ExistingQueue_Updates()
    {
        var queue = new PlaybackQueue
        {
            UserId = 2,
            DeviceName = "Desktop",
            QueueData = "[{\"mediaItemId\":1}]",
            CurrentIndex = 0
        };

        await _repo.UpsertAsync(queue);

        queue.QueueData = "[{\"mediaItemId\":1},{\"mediaItemId\":2}]";
        queue.CurrentIndex = 1;
        await _repo.UpsertAsync(queue);

        var result = await _repo.GetByUserAndDeviceAsync(2, "Desktop");
        Assert.NotNull(result);
        Assert.Contains("mediaItemId\":2", result.QueueData);
        Assert.Equal(1, result.CurrentIndex);
    }

    [Fact]
    public async Task GetByUserAsync_ReturnsAllDevices()
    {
        await _repo.UpsertAsync(new PlaybackQueue
        {
            UserId = 3,
            DeviceName = "Phone",
            QueueData = "[]",
            UpdatedAt = DateTime.UtcNow
        });

        await _repo.UpsertAsync(new PlaybackQueue
        {
            UserId = 3,
            DeviceName = "Desktop",
            QueueData = "[]",
            UpdatedAt = DateTime.UtcNow
        });

        var result = await _repo.GetByUserAsync(3);
        Assert.Equal(2, result.Count);
    }

    [Fact]
    public async Task GetByUserAsync_UserIsolation()
    {
        await _repo.UpsertAsync(new PlaybackQueue
        {
            UserId = 10,
            DeviceName = "Device",
            QueueData = "[]",
            UpdatedAt = DateTime.UtcNow
        });

        await _repo.UpsertAsync(new PlaybackQueue
        {
            UserId = 11,
            DeviceName = "Device",
            QueueData = "[]",
            UpdatedAt = DateTime.UtcNow
        });

        var user10 = await _repo.GetByUserAsync(10);
        Assert.Single(user10);

        var user11 = await _repo.GetByUserAsync(11);
        Assert.Single(user11);
    }
}
