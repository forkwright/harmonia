// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later
using System.Net;
namespace Mouseion.Api.Tests.Authentication.Oidc;
public class OidcControllerTests : ControllerTestBase
{
    public OidcControllerTests(TestWebApplicationFactory factory) : base(factory) { }
    [Fact] public async Task GetProviders_ReturnsSuccess() { var r = await Client.GetAsync("/api/v3/auth/oidc/providers"); r.EnsureSuccessStatusCode(); }
    [Fact] public async Task Authorize_InvalidSlug_ReturnsBadRequest() { var r = await Client.GetAsync("/api/v3/auth/oidc/authorize/nonexistent"); Assert.True(r.StatusCode == HttpStatusCode.BadRequest || r.StatusCode == HttpStatusCode.NotFound); }
    [Fact] public async Task Callback_InvalidState_ReturnsUnauthorized() { var r = await Client.GetAsync("/api/v3/auth/oidc/callback?state=invalid&code=test"); Assert.Equal(HttpStatusCode.Unauthorized, r.StatusCode); }
    [Fact] public async Task Callback_WithError_ReturnsBadRequest() { var r = await Client.GetAsync("/api/v3/auth/oidc/callback?state=x&code=y&error=access_denied"); Assert.Equal(HttpStatusCode.BadRequest, r.StatusCode); }
}
