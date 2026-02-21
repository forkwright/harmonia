// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Api.Progress;
using Mouseion.Api.Tests;

namespace Mouseion.Api.Tests.Progress;

public class ProgressControllerTests : ControllerTestBase, IClassFixture<TestWebApplicationFactory>
{
    public ProgressControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task GetProgress_NonExistentMediaItem_Returns404()
    {
        var response = await Client.GetAsync("/api/v3/progress/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task UpdateProgress_NonExistentMediaItem_Returns404()
    {
        var request = new UpdateProgressRequest
        {
            MediaItemId = 99999,
            PositionMs = 5000,
            TotalDurationMs = 100000,
            IsComplete = false
        };

        var response = await Client.PostAsJsonAsync("/api/v3/progress", request);
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task DeleteProgress_NonExistentMediaItem_Returns204()
    {
        // Delete is idempotent — even if no progress exists, it returns 204
        var response = await Client.DeleteAsync("/api/v3/progress/99999");
        Assert.Equal(HttpStatusCode.NoContent, response.StatusCode);
    }

    [Fact]
    public async Task GetRecentlyPlayed_EmptyDatabase_ReturnsEmptyList()
    {
        var response = await Client.GetAsync("/api/v3/progress/recent");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);

        var items = await response.Content.ReadFromJsonAsync<List<MediaProgressResource>>();
        Assert.NotNull(items);
        Assert.Empty(items);
    }

    [Fact]
    public async Task GetRecentlyPlayed_WithLimit_RespectsLimit()
    {
        var response = await Client.GetAsync("/api/v3/progress/recent?limit=5");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);

        var items = await response.Content.ReadFromJsonAsync<List<MediaProgressResource>>();
        Assert.NotNull(items);
        Assert.True(items.Count <= 5);
    }

    [Fact]
    public async Task UpdateProgressBatch_EmptyList_ReturnsEmptyList()
    {
        var requests = new List<UpdateProgressRequest>();
        var response = await Client.PostAsJsonAsync("/api/v3/progress/batch", requests);
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);

        var items = await response.Content.ReadFromJsonAsync<List<MediaProgressResource>>();
        Assert.NotNull(items);
        Assert.Empty(items);
    }
}
