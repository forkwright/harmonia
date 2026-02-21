// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Api.Streaming;
using Mouseion.Api.Tests;

namespace Mouseion.Api.Tests.Streaming;

public class StreamingControllerTests : ControllerTestBase, IClassFixture<TestWebApplicationFactory>
{
    public StreamingControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task StreamMedia_NonExistentFile_Returns404()
    {
        var response = await Client.GetAsync("/api/v3/stream/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task StreamTranscoded_NonExistentFile_Returns404()
    {
        var response = await Client.GetAsync("/api/v3/stream/99999/transcode");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task StreamTranscoded_WithFormatParam_Returns404ForMissing()
    {
        var response = await Client.GetAsync("/api/v3/stream/99999/transcode?format=opus&bitrate=128000");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task GetCover_NonExistentMediaItem_Returns404()
    {
        var response = await Client.GetAsync("/api/v3/mediacover/99999/poster");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task GetCover_WithResizeParams_Returns404ForMissing()
    {
        var response = await Client.GetAsync("/api/v3/mediacover/99999/poster?width=200&height=300");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Theory]
    [InlineData("poster")]
    [InlineData("banner")]
    [InlineData("fanart")]
    [InlineData("logo")]
    public async Task GetCover_AllCoverTypes_Returns404ForMissing(string coverType)
    {
        var response = await Client.GetAsync($"/api/v3/mediacover/1/{coverType}");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }
}
