// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later
using System.Net;
namespace Mouseion.Api.Tests.News;
public class NewsArticlesControllerTests : ControllerTestBase
{
    public NewsArticlesControllerTests(TestWebApplicationFactory factory) : base(factory) { }
    [Fact] public async Task GetArticles_ReturnsSuccess() { var r = await Client.GetAsync("/api/v3/articles"); r.EnsureSuccessStatusCode(); }
    [Fact] public async Task GetArticles_UnreadFilter_ReturnsSuccess() { var r = await Client.GetAsync("/api/v3/articles?unreadOnly=true"); r.EnsureSuccessStatusCode(); }
    [Fact] public async Task GetArticles_StarredFilter_ReturnsSuccess() { var r = await Client.GetAsync("/api/v3/articles?starredOnly=true"); r.EnsureSuccessStatusCode(); }
}
