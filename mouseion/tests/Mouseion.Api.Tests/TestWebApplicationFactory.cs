// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Hosting;
using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Mouseion.Common.EnvironmentInfo;

namespace Mouseion.Api.Tests;

public class TestWebApplicationFactory : WebApplicationFactory<Program>
{
    private readonly string _testDir;

    public TestWebApplicationFactory()
    {
        // Use temp directory with unique GUID for each factory instance
        _testDir = Path.Combine(Path.GetTempPath(), $"mouseion_test_{Guid.NewGuid()}");

        // Set environment variable for AppFolderInfo to use
        Environment.SetEnvironmentVariable("MOUSEION_TEST_APPDATA", _testDir);
    }

    protected override void ConfigureWebHost(IWebHostBuilder builder)
    {
        builder.ConfigureAppConfiguration((context, config) =>
        {
            // Add test configuration
            config.AddInMemoryCollection(new Dictionary<string, string?>
            {
                ["ApiKey"] = "test-api-key",
                ["AllowedOrigins:0"] = "http://localhost"
            });
        });

        builder.UseEnvironment("Test");
    }

    protected override void Dispose(bool disposing)
    {
        if (disposing && Directory.Exists(_testDir))
        {
            try
            {
                Directory.Delete(_testDir, recursive: true);
            }
            catch
            {
                // Ignore cleanup errors
            }
        }

        // Clear environment variable
        Environment.SetEnvironmentVariable("MOUSEION_TEST_APPDATA", null);

        base.Dispose(disposing);
    }
}
