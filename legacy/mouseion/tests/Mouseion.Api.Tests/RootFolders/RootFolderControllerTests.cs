// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later
using System.Net;
namespace Mouseion.Api.Tests.RootFolders;
public class RootFolderControllerTests : ControllerTestBase
{
    public RootFolderControllerTests(TestWebApplicationFactory factory) : base(factory) { }
    [Fact] public async Task GetRootFolders_ReturnsSuccess() { var r = await Client.GetAsync("/api/v3/rootfolders"); r.EnsureSuccessStatusCode(); }
    [Fact] public async Task GetRootFolder_NonExistent_ReturnsNotFound() { var r = await Client.GetAsync("/api/v3/rootfolders/99999"); Assert.Equal(HttpStatusCode.NotFound, r.StatusCode); }
    [Fact] public async Task GetRootFolders_WithMediaType_ReturnsSuccess() { var r = await Client.GetAsync("/api/v3/rootfolders?mediaType=0"); r.EnsureSuccessStatusCode(); }
}
