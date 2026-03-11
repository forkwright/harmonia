// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Api.Progress;
using Mouseion.Api.Tests;

namespace Mouseion.Api.Tests.Progress;

public class ContinueWatchingControllerTests : ControllerTestBase, IClassFixture<TestWebApplicationFactory>
{
    public ContinueWatchingControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task GetContinue_EmptyDatabase_ReturnsEmptyList()
    {
        var response = await Client.GetAsync("/api/v3/continue");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);

        var items = await response.Content.ReadFromJsonAsync<List<ContinueResource>>();
        Assert.NotNull(items);
        Assert.Empty(items);
    }

    [Fact]
    public async Task GetContinue_WithLimit_RespectsLimit()
    {
        var response = await Client.GetAsync("/api/v3/continue?limit=5");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);

        var items = await response.Content.ReadFromJsonAsync<List<ContinueResource>>();
        Assert.NotNull(items);
        Assert.True(items.Count <= 5);
    }

    [Fact]
    public async Task GetContinue_WithMediaTypeFilter_ReturnsFiltered()
    {
        var response = await Client.GetAsync("/api/v3/continue?mediaType=Movie");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);

        var items = await response.Content.ReadFromJsonAsync<List<ContinueResource>>();
        Assert.NotNull(items);
        // All items (if any) should be of the requested type
        Assert.All(items, item => Assert.Equal("Movie", item.MediaType));
    }
}
