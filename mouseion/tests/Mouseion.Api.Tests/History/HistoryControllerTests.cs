// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;

namespace Mouseion.Api.Tests.History;

public class HistoryControllerTests : ControllerTestBase
{
    public HistoryControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task GetAll_ReturnsSuccess()
    {
        var response = await Client.GetAsync("/api/v3/history");
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task GetAll_WithPagination_ReturnsSuccess()
    {
        var response = await Client.GetAsync("/api/v3/history?page=1&pageSize=5");
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task GetByMediaItem_WithNonExistentId_ReturnsSuccess()
    {
        var response = await Client.GetAsync("/api/v3/history/mediaitem/99999");
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task GetSince_WithDate_ReturnsSuccess()
    {
        var response = await Client.GetAsync("/api/v3/history/since?date=2025-01-01");
        response.EnsureSuccessStatusCode();
    }
}
