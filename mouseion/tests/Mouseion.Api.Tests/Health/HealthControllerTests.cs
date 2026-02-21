// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Moq;
using Mouseion.Api.Health;
using Mouseion.Core.HealthCheck;

namespace Mouseion.Api.Tests.Health;

public class HealthControllerTests
{
    private readonly Mock<IHealthCheckService> _healthCheckService;
    private readonly HealthController _controller;

    public HealthControllerTests()
    {
        _healthCheckService = new Mock<IHealthCheckService>();
        _controller = new HealthController(_healthCheckService.Object);
    }

    [Fact]
    public void GetHealth_ReturnsOk_WithHealthChecks()
    {
        var checks = new List<Core.HealthCheck.HealthCheck>
        {
            new()
            {
                Type = HealthCheckType.Warning,
                Message = "Disk space low",
                WikiUrl = "https://wiki.example.com/disk"
            },
            new()
            {
                Type = HealthCheckType.Error,
                Message = "Database connection failed",
                WikiUrl = null
            }
        };

        _healthCheckService.Setup(s => s.PerformHealthChecks()).Returns(checks);

        var result = _controller.GetHealth();

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<HealthResource>>(okResult.Value);
        Assert.Equal(2, resources.Count);
        Assert.Equal("Warning", resources[0].Type);
        Assert.Equal("Disk space low", resources[0].Message);
        Assert.Equal("https://wiki.example.com/disk", resources[0].WikiUrl);
        Assert.Equal("Error", resources[1].Type);
        Assert.Null(resources[1].WikiUrl);
    }

    [Fact]
    public void GetHealth_ReturnsEmptyList_WhenHealthy()
    {
        _healthCheckService.Setup(s => s.PerformHealthChecks())
            .Returns(new List<Core.HealthCheck.HealthCheck>());

        var result = _controller.GetHealth();

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<HealthResource>>(okResult.Value);
        Assert.Empty(resources);
    }
}
