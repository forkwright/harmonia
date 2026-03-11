// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.HealthCheck;

namespace Mouseion.Api.Health;

[ApiController]
[Route("api/v3/[controller]")]
[Authorize]
public class HealthController : ControllerBase
{
    private readonly IHealthCheckService _healthCheckService;

    public HealthController(IHealthCheckService healthCheckService)
    {
        _healthCheckService = healthCheckService;
    }

    [HttpGet]
    public ActionResult<List<HealthResource>> GetHealth()
    {
        var healthChecks = _healthCheckService.PerformHealthChecks();
        var resources = healthChecks.Select(MapToResource).ToList();
        return Ok(resources);
    }

    private static HealthResource MapToResource(Core.HealthCheck.HealthCheck check)
    {
        return new HealthResource
        {
            Type = check.Type.ToString(),
            Message = check.Message,
            WikiUrl = check.WikiUrl
        };
    }
}
