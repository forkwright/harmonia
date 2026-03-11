// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;

namespace Mouseion.Api.Tests.Comic;

public class ComicSeriesControllerTests : ControllerTestBase
{
    public ComicSeriesControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task GetSeries_ReturnsSuccess()
    {
        var response = await Client.GetAsync("/api/v3/comic");
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task GetSeries_WithPagination_ReturnsSuccess()
    {
        var response = await Client.GetAsync("/api/v3/comic?page=1&pageSize=10");
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task GetSeries_WithNonExistentId_ReturnsNotFound()
    {
        var response = await Client.GetAsync("/api/v3/comic/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }
}
