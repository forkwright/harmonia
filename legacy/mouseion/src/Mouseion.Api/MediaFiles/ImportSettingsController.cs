// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.MediaFiles.Import;

namespace Mouseion.Api.MediaFiles;

[ApiController]
[Route("api/v3/import")]
[Authorize]
public class ImportSettingsController : ControllerBase
{
    /// <summary>
    /// Get current import settings and available strategies
    /// </summary>
    /// <returns>Import settings</returns>
    [HttpGet("settings")]
    public ActionResult<ImportSettingsResource> GetSettings()
    {
        var settings = new ImportSettings();

        return Ok(new ImportSettingsResource
        {
            DefaultStrategy = settings.DefaultStrategy.ToString(),
            VerifyChecksum = settings.VerifyChecksum,
            PreserveTimestamps = settings.PreserveTimestamps,
            AvailableStrategies = settings.AvailableStrategies.Select(s => s.ToString()).ToList()
        });
    }
}

public class ImportSettingsResource
{
    public string DefaultStrategy { get; set; } = null!;
    public bool VerifyChecksum { get; set; }
    public bool PreserveTimestamps { get; set; }
    public List<string> AvailableStrategies { get; set; } = new();
}
