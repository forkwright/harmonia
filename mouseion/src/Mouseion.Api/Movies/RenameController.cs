// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Movies.Organization;

namespace Mouseion.Api.Movies;

[ApiController]
[Route("api/v3/movies")]
[Authorize]
public class RenameController : ControllerBase
{
    private readonly IFileOrganizationService _organizationService;

    public RenameController(IFileOrganizationService organizationService)
    {
        _organizationService = organizationService;
    }

    [HttpPost("{id:int}/rename")]
    public async Task<ActionResult<OrganizationResultResource>> RenameMovie(
        int id,
        [FromBody] RenameRequest request,
        CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(request.NamingPattern))
        {
            return BadRequest(new { error = "Naming pattern is required" });
        }

        var result = await _organizationService.RenameMovieAsync(id, request.NamingPattern, request.Strategy, ct).ConfigureAwait(false);

        if (!result.Success)
        {
            return BadRequest(new { error = result.ErrorMessage });
        }

        return Ok(ToResource(result));
    }

    [HttpPost("{id:int}/rename/preview")]
    public async Task<ActionResult<OrganizationResultResource>> PreviewRename(
        int id,
        [FromBody] RenameRequest request,
        CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(request.NamingPattern))
        {
            return BadRequest(new { error = "Naming pattern is required" });
        }

        var result = await _organizationService.PreviewRenameAsync(id, request.NamingPattern, ct).ConfigureAwait(false);

        if (!result.Success)
        {
            return BadRequest(new { error = result.ErrorMessage });
        }

        return Ok(ToResource(result));
    }

    private static OrganizationResultResource ToResource(OrganizationResult result)
    {
        return new OrganizationResultResource
        {
            Success = result.Success,
            OriginalPath = result.OriginalPath,
            NewPath = result.NewPath,
            ErrorMessage = result.ErrorMessage,
            StrategyUsed = result.StrategyUsed.ToString(),
            IsDryRun = result.IsDryRun
        };
    }
}

public class RenameRequest
{
    public string NamingPattern { get; set; } = "{Movie Title} ({Movie Year}) - {Quality}{File Extension}";
    public FileStrategy Strategy { get; set; } = FileStrategy.Hardlink;
}

public class OrganizationResultResource
{
    public bool Success { get; set; }
    public string OriginalPath { get; set; } = string.Empty;
    public string NewPath { get; set; } = string.Empty;
    public string? ErrorMessage { get; set; }
    public string StrategyUsed { get; set; } = string.Empty;
    public bool IsDryRun { get; set; }
}
