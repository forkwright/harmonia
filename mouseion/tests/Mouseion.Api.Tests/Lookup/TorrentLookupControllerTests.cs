// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;

namespace Mouseion.Api.Tests.Lookup;

public class TorrentLookupControllerTests : ControllerTestBase
{
    public TorrentLookupControllerTests(TestWebApplicationFactory factory) : base(factory)
    {
    }

    [Fact]
    public async Task SearchBooks_WithNoParameters_ReturnsClientError()
    {
        var response = await Client.GetAsync("/api/v3/lookup/torrents/books");

        // Should return a client error (4xx) when no parameters provided
        var statusCode = (int)response.StatusCode;
        Assert.True(statusCode >= 400 && statusCode < 500, $"Expected 4xx error, got {response.StatusCode}");
    }

    [Fact]
    public async Task SearchBooks_WithTitle_EndpointExists()
    {
        var response = await Client.GetAsync("/api/v3/lookup/torrents/books?title=test");

        // Endpoint exists if we don't get 404 - external service may fail
        Assert.NotEqual(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task SearchBooks_WithAuthor_EndpointExists()
    {
        var response = await Client.GetAsync("/api/v3/lookup/torrents/books?author=test");

        Assert.NotEqual(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task SearchAudiobooks_WithNoParameters_ReturnsClientError()
    {
        var response = await Client.GetAsync("/api/v3/lookup/torrents/audiobooks");

        // Should return a client error (4xx) when no parameters provided
        var statusCode = (int)response.StatusCode;
        Assert.True(statusCode >= 400 && statusCode < 500, $"Expected 4xx error, got {response.StatusCode}");
    }

    [Fact]
    public async Task SearchAudiobooks_WithTitle_EndpointExists()
    {
        var response = await Client.GetAsync("/api/v3/lookup/torrents/audiobooks?title=test");

        Assert.NotEqual(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task SearchAudiobooks_WithAuthor_EndpointExists()
    {
        var response = await Client.GetAsync("/api/v3/lookup/torrents/audiobooks?author=test");

        Assert.NotEqual(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task SearchAudiobooks_WithNarrator_EndpointExists()
    {
        var response = await Client.GetAsync("/api/v3/lookup/torrents/audiobooks?narrator=test");

        Assert.NotEqual(HttpStatusCode.NotFound, response.StatusCode);
    }
}
