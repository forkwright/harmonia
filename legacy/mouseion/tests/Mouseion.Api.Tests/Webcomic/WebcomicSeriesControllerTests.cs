// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;

namespace Mouseion.Api.Tests.Webcomic;

public class WebcomicSeriesControllerTests : ControllerTestBase
{
    public WebcomicSeriesControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task GetSeries_ReturnsSuccess()
    {
        var response = await Client.GetAsync("/api/v3/webcomic");
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task GetSeries_WithPagination_ReturnsSuccess()
    {
        var response = await Client.GetAsync("/api/v3/webcomic?page=1&pageSize=10");
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task GetSeries_WithNonExistentId_ReturnsNotFound()
    {
        var response = await Client.GetAsync("/api/v3/webcomic/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }
}
