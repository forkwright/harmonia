// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;

namespace Mouseion.Api.Tests.Notifications;

public class NotificationControllerTests : ControllerTestBase
{
    public NotificationControllerTests(TestWebApplicationFactory factory) : base(factory) { }

    [Fact]
    public async Task List_ReturnsSuccess()
    {
        var response = await Client.GetAsync("/api/v3/notifications");
        response.EnsureSuccessStatusCode();
    }

    [Fact]
    public async Task Get_WithNonExistentId_ReturnsNotFound()
    {
        var response = await Client.GetAsync("/api/v3/notifications/99999");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }
}
