// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Common.EnvironmentInfo;
using Mouseion.Core.Datastore;

namespace Mouseion.Api.Health;

[ApiController]
public class PingController : ControllerBase
{
    private readonly IDatabase _database;

    public PingController(IDatabase database)
    {
        _database = database;
    }

    /// <summary>
    /// Lightweight health check. No authentication required.
    /// Returns 200 with status if healthy, 503 if DB unreachable.
    /// Used by Docker HEALTHCHECK and Portainer monitoring.
    /// </summary>
    [HttpGet("/ping")]
    [AllowAnonymous]
    public ActionResult Ping()
    {
        try
        {
            // Verify DB is reachable with a minimal query
            using var conn = _database.OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "SELECT 1";
            cmd.ExecuteScalar();

            return Ok(new PingResponse
            {
                Status = "ok",
                Version = BuildInfo.Version?.ToString() ?? "unknown",
                Database = "connected"
            });
        }
        catch (Exception)
        {
            return StatusCode(503, new PingResponse
            {
                Status = "degraded",
                Version = BuildInfo.Version?.ToString() ?? "unknown",
                Database = "unreachable"
            });
        }
    }
}

public class PingResponse
{
    public string Status { get; set; } = null!;
    public string Version { get; set; } = null!;
    public string Database { get; set; } = null!;
}
