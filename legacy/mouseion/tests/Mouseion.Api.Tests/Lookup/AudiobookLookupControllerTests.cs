// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;

namespace Mouseion.Api.Tests.Lookup;

public class AudiobookLookupControllerTests : ControllerTestBase
{
    public AudiobookLookupControllerTests(TestWebApplicationFactory factory) : base(factory)
    {
    }

    [Fact]
    public async Task Search_WithNoParameters_ReturnsClientError()
    {
        var response = await Client.GetAsync("/api/v3/lookup/audiobooks");

        // Should return a client error (4xx) when no parameters provided
        var statusCode = (int)response.StatusCode;
        Assert.True(statusCode >= 400 && statusCode < 500, $"Expected 4xx error, got {response.StatusCode}");
    }

    [Fact]
    public async Task Search_WithTitle_EndpointExists()
    {
        var response = await Client.GetAsync("/api/v3/lookup/audiobooks?title=test");

        // Endpoint exists if we don't get 404 - external service may fail
        Assert.NotEqual(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task Search_WithAuthor_EndpointExists()
    {
        var response = await Client.GetAsync("/api/v3/lookup/audiobooks?author=test");

        Assert.NotEqual(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task Search_WithNarrator_EndpointExists()
    {
        var response = await Client.GetAsync("/api/v3/lookup/audiobooks?narrator=test");

        Assert.NotEqual(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task GetByAsin_EndpointExists()
    {
        var response = await Client.GetAsync("/api/v3/lookup/audiobooks/B0TESTASIN");

        Assert.NotEqual(HttpStatusCode.MethodNotAllowed, response.StatusCode);
    }

    [Fact]
    public async Task SearchByNarrator_EndpointExists()
    {
        var response = await Client.GetAsync("/api/v3/lookup/audiobooks/narrator/testnarrator");

        Assert.NotEqual(HttpStatusCode.MethodNotAllowed, response.StatusCode);
    }
}
