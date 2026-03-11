// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Api.MediaFiles;

namespace Mouseion.Api.Tests.MediaFiles;

public class ImportSettingsControllerTests : ControllerTestBase
{
    public ImportSettingsControllerTests(TestWebApplicationFactory factory) : base(factory)
    {
    }

    [Fact]
    public async Task GetSettings_ReturnsSuccessfully()
    {
        var response = await Client.GetAsync("/api/v3/import/settings");
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<ImportSettingsResource>();
        Assert.NotNull(result);
    }

    [Fact]
    public async Task GetSettings_ReturnsDefaultStrategy()
    {
        var response = await Client.GetAsync("/api/v3/import/settings");
        var result = await response.Content.ReadFromJsonAsync<ImportSettingsResource>();

        Assert.NotNull(result);
        Assert.Equal("Hardlink", result.DefaultStrategy);
    }

    [Fact]
    public async Task GetSettings_ReturnsVerificationEnabled()
    {
        var response = await Client.GetAsync("/api/v3/import/settings");
        var result = await response.Content.ReadFromJsonAsync<ImportSettingsResource>();

        Assert.NotNull(result);
        Assert.True(result.VerifyChecksum);
    }

    [Fact]
    public async Task GetSettings_ReturnsPreserveTimestampsEnabled()
    {
        var response = await Client.GetAsync("/api/v3/import/settings");
        var result = await response.Content.ReadFromJsonAsync<ImportSettingsResource>();

        Assert.NotNull(result);
        Assert.True(result.PreserveTimestamps);
    }

    [Fact]
    public async Task GetSettings_ReturnsAvailableStrategies()
    {
        var response = await Client.GetAsync("/api/v3/import/settings");
        var result = await response.Content.ReadFromJsonAsync<ImportSettingsResource>();

        Assert.NotNull(result);
        Assert.NotNull(result.AvailableStrategies);
        Assert.Contains("Hardlink", result.AvailableStrategies);
        Assert.Contains("Copy", result.AvailableStrategies);
        Assert.Contains("Move", result.AvailableStrategies);
        Assert.Equal(3, result.AvailableStrategies.Count);
    }

    [Fact]
    public async Task GetSettings_ReturnsOkStatus()
    {
        var response = await Client.GetAsync("/api/v3/import/settings");

        Assert.Equal(HttpStatusCode.OK, response.StatusCode);
    }
}
