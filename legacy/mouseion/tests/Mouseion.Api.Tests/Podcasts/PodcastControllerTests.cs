// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;

namespace Mouseion.Api.Tests.Podcasts;

public class PodcastControllerTests : ControllerTestBase
{
    public PodcastControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task GetPodcasts_ReturnsSuccess()
    {
        var response = await Client.GetAsync("/api/v3/podcasts");
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task GetPodcasts_WithPagination_ReturnsSuccess()
    {
        var response = await Client.GetAsync("/api/v3/podcasts?page=1&pageSize=10");
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task GetPodcast_WithNonExistentId_ReturnsNotFound()
    {
        var response = await Client.GetAsync("/api/v3/podcasts/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }
}
