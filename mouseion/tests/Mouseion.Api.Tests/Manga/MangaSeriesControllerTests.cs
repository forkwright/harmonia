// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;

namespace Mouseion.Api.Tests.Manga;

public class MangaSeriesControllerTests : ControllerTestBase
{
    public MangaSeriesControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task GetSeries_ReturnsSuccess()
    {
        var response = await Client.GetAsync("/api/v3/manga");
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task GetSeries_WithPagination_ReturnsSuccess()
    {
        var response = await Client.GetAsync("/api/v3/manga?page=1&pageSize=10");
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task GetSeries_WithNonExistentId_ReturnsNotFound()
    {
        var response = await Client.GetAsync("/api/v3/manga/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }
}
