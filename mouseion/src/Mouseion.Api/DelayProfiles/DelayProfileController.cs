// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.Extensions.Logging;
using Mouseion.Core.Download.DelayProfiles;

namespace Mouseion.Api.DelayProfiles;

/// <summary>
/// API controller for delay profiles — quality-conscious acquisition delays that wait
/// for better releases before grabbing.
/// </summary>
[ApiController]
[Route("api/v3/delayprofiles")]
[Authorize]
public class DelayProfileController : ControllerBase
{
    private readonly IDelayProfileService _service;
    private readonly ILogger<DelayProfileController> _logger;

    public DelayProfileController(IDelayProfileService service, ILogger<DelayProfileController> logger)
    {
        _service = service;
        _logger = logger;
    }

    /// <summary>
    /// List all delay profiles, ordered by priority.
    /// </summary>
    [HttpGet]
    public async Task<ActionResult<List<DelayProfileResource>>> List(CancellationToken ct)
    {
        var profiles = await _service.GetAllAsync(ct).ConfigureAwait(false);
        return Ok(profiles.Select(p => p.ToResource()).OrderBy(p => p.Order).ToList());
    }

    /// <summary>
    /// Get a specific delay profile by ID.
    /// </summary>
    [HttpGet("{id:int}")]
    public async Task<ActionResult<DelayProfileResource>> Get(int id, CancellationToken ct)
    {
        var profile = await _service.GetAsync(id, ct).ConfigureAwait(false);
        if (profile == null) return NotFound();
        return Ok(profile.ToResource());
    }

    /// <summary>
    /// Create a new delay profile.
    /// </summary>
    [HttpPost]
    public async Task<ActionResult<DelayProfileResource>> Create(
        [FromBody][Required] DelayProfileResource resource, CancellationToken ct)
    {
        var entity = resource.ToModel();
        var created = await _service.CreateAsync(entity, ct).ConfigureAwait(false);
        return CreatedAtAction(nameof(Get), new { id = created.Id }, created.ToResource());
    }

    /// <summary>
    /// Update an existing delay profile.
    /// </summary>
    [HttpPut("{id:int}")]
    public async Task<ActionResult<DelayProfileResource>> Update(
        int id, [FromBody][Required] DelayProfileResource resource, CancellationToken ct)
    {
        var existing = await _service.GetAsync(id, ct).ConfigureAwait(false);
        if (existing == null) return NotFound();

        var entity = resource.ToModel();
        entity.Id = id;
        var updated = await _service.UpdateAsync(entity, ct).ConfigureAwait(false);
        return Ok(updated.ToResource());
    }

    /// <summary>
    /// Delete a delay profile.
    /// </summary>
    [HttpDelete("{id:int}")]
    public async Task<ActionResult> Delete(int id, CancellationToken ct)
    {
        var existing = await _service.GetAsync(id, ct).ConfigureAwait(false);
        if (existing == null) return NotFound();

        await _service.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    /// <summary>
    /// Test delay evaluation for given parameters — useful for debugging profile behavior.
    /// </summary>
    [HttpPost("test")]
    public async Task<ActionResult<DelayEvaluation>> TestEvaluation(
        [FromBody][Required] DelayTestRequest request, CancellationToken ct)
    {
        var result = await _service.EvaluateAsync(
            request.MediaType,
            request.Protocol,
            request.QualityWeight,
            request.HasPreferredWords,
            request.TagIds ?? Array.Empty<int>(),
            ct).ConfigureAwait(false);

        return Ok(result);
    }
}

public class DelayTestRequest
{
    public Core.MediaTypes.MediaType MediaType { get; set; }
    public Core.Download.DownloadProtocol Protocol { get; set; }
    public int QualityWeight { get; set; }
    public bool HasPreferredWords { get; set; }
    public int[]? TagIds { get; set; }
}
