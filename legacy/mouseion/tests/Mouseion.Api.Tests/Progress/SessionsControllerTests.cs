// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Api.Progress;
using Mouseion.Api.Tests;

namespace Mouseion.Api.Tests.Progress;

public class SessionsControllerTests : ControllerTestBase, IClassFixture<TestWebApplicationFactory>
{
    public SessionsControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task GetSessions_EmptyDatabase_ReturnsEmptyList()
    {
        var response = await Client.GetAsync("/api/v3/sessions");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);

        var items = await response.Content.ReadFromJsonAsync<List<PlaybackSessionResource>>();
        Assert.NotNull(items);
        Assert.Empty(items);
    }

    [Fact]
    public async Task GetSessions_ActiveOnly_ReturnsEmptyList()
    {
        var response = await Client.GetAsync("/api/v3/sessions?activeOnly=true");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);

        var items = await response.Content.ReadFromJsonAsync<List<PlaybackSessionResource>>();
        Assert.NotNull(items);
        Assert.Empty(items);
    }

    [Fact]
    public async Task GetSession_NonExistentId_Returns404()
    {
        var response = await Client.GetAsync("/api/v3/sessions/non-existent-session-id");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task StartSession_NonExistentMediaItem_Returns404()
    {
        var request = new StartSessionRequest
        {
            MediaItemId = 99999,
            DeviceName = "Test Device",
            DeviceType = "Desktop"
        };

        var response = await Client.PostAsJsonAsync("/api/v3/sessions", request);
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task UpdateSession_NonExistentId_Returns404()
    {
        var request = new UpdateSessionRequest { EndSession = true, EndPositionMs = 5000 };
        var response = await Client.PutAsJsonAsync("/api/v3/sessions/non-existent", request);
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task DeleteSession_NonExistentId_Returns404()
    {
        var response = await Client.DeleteAsync("/api/v3/sessions/non-existent");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task GetSessionsByMediaItem_EmptyDatabase_ReturnsEmptyList()
    {
        var response = await Client.GetAsync("/api/v3/sessions/media/99999");
        Assert.Equal(HttpStatusCode.OK, response.StatusCode);

        var items = await response.Content.ReadFromJsonAsync<List<PlaybackSessionResource>>();
        Assert.NotNull(items);
        Assert.Empty(items);
    }
}
