// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;

namespace Mouseion.Api.Tests.ImportLists;

public class ImportListControllerTests : ControllerTestBase
{
    public ImportListControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task GetAll_ReturnsSuccess()
    {
        var response = await Client.GetAsync("/api/v3/importlist");
        response.EnsureSuccessStatusCode();
    }
}
