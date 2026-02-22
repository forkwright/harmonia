// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later
using System.Net;
namespace Mouseion.Api.Tests.SystemStatus;
public class SystemControllerTests : ControllerTestBase
{
    public SystemControllerTests(TestWebApplicationFactory factory) : base(factory) { }
    [Fact] public async Task GetStatus_ReturnsSuccess() { var r = await Client.GetAsync("/api/v3/system/status"); r.EnsureSuccessStatusCode(); }
}
