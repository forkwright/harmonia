// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later
using System.Net;
namespace Mouseion.Api.Tests.News;
public class NewsFeedsControllerTests : ControllerTestBase
{
    public NewsFeedsControllerTests(TestWebApplicationFactory factory) : base(factory) { }
    [Fact] public async Task GetFeeds_ReturnsSuccess() { var r = await Client.GetAsync("/api/v3/feeds"); r.EnsureSuccessStatusCode(); }
    [Fact] public async Task GetFeed_NonExistent_ReturnsNotFound() { var r = await Client.GetAsync("/api/v3/feeds/99999"); Assert.Equal(HttpStatusCode.NotFound, r.StatusCode); }
}
