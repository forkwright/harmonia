// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Api.Progress;
using Mouseion.Api.Tests;
using Mouseion.Core.Progress;

namespace Mouseion.Api.Tests.Progress;

public class QueueControllerTests : ControllerTestBase, IClassFixture<TestWebApplicationFactory>
{
    public QueueControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task GetQueues_EmptyDatabase_ReturnsEmptyList()
    {
        var response = await Client.GetAsync("/api/v3/queue");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);

        var items = await response.Content.ReadFromJsonAsync<List<PlaybackQueueResource>>();
        Assert.NotNull(items);
        Assert.Empty(items);
    }

    [Fact]
    public async Task GetQueues_ByDeviceName_NoMatch_ReturnsEmptyList()
    {
        var response = await Client.GetAsync("/api/v3/queue?deviceName=NonExistent");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);

        var items = await response.Content.ReadFromJsonAsync<List<PlaybackQueueResource>>();
        Assert.NotNull(items);
        Assert.Empty(items);
    }

    [Fact]
    public async Task SaveQueue_ValidRequest_Returns200()
    {
        var request = new SaveQueueRequest
        {
            DeviceName = "TestDevice",
            Items = new List<QueueItem>
            {
                new QueueItem { MediaItemId = 1, Title = "Test Item", MediaType = "Movie" }
            },
            CurrentIndex = 0,
            ShuffleEnabled = false,
            RepeatMode = "none"
        };

        var response = await Client.PutAsJsonAsync("/api/v3/queue", request);
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);

        var result = await response.Content.ReadFromJsonAsync<PlaybackQueueResource>();
        Assert.NotNull(result);
        Assert.Equal("TestDevice", result.DeviceName);
        Assert.Single(result.Items);
    }

    [Fact]
    public async Task SaveQueue_UpsertSameDevice_UpdatesExisting()
    {
        // First save
        var request1 = new SaveQueueRequest
        {
            DeviceName = "UpsertDevice",
            Items = new List<QueueItem>
            {
                new QueueItem { MediaItemId = 1, Title = "Item 1" }
            },
            CurrentIndex = 0
        };
        await Client.PutAsJsonAsync("/api/v3/queue", request1);

        // Second save (upsert)
        var request2 = new SaveQueueRequest
        {
            DeviceName = "UpsertDevice",
            Items = new List<QueueItem>
            {
                new QueueItem { MediaItemId = 1, Title = "Item 1" },
                new QueueItem { MediaItemId = 2, Title = "Item 2" }
            },
            CurrentIndex = 1
        };
        var response = await Client.PutAsJsonAsync("/api/v3/queue", request2);
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);

        var result = await response.Content.ReadFromJsonAsync<PlaybackQueueResource>();
        Assert.NotNull(result);
        Assert.Equal(2, result.Items.Count);
        Assert.Equal(1, result.CurrentIndex);
    }

    [Fact]
    public async Task TransferPlayback_NoSourceQueue_Returns404()
    {
        var request = new PlaybackTransferRequest
        {
            FromDevice = "NonExistent",
            ToDevice = "TargetDevice"
        };

        var response = await Client.PostAsJsonAsync("/api/v3/queue/transfer", request);
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task DeleteQueue_NonExistentDevice_Returns404()
    {
        var response = await Client.DeleteAsync("/api/v3/queue?deviceName=NonExistent");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }
}
