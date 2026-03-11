// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.MediaItems;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Tags.AutoTagging;

namespace Mouseion.Api.AutoTagging;

[ApiController]
[Route("api/v3/autotagging")]
[Authorize]
public class AutoTaggingController : ControllerBase
{
    private readonly IAutoTaggingRuleRepository _ruleRepository;
    private readonly IAutoTaggingService _autoTaggingService;
    private readonly IMediaItemRepository _mediaItemRepository;

    public AutoTaggingController(
        IAutoTaggingRuleRepository ruleRepository,
        IAutoTaggingService autoTaggingService,
        IMediaItemRepository mediaItemRepository)
    {
        _ruleRepository = ruleRepository;
        _autoTaggingService = autoTaggingService;
        _mediaItemRepository = mediaItemRepository;
    }

    [HttpGet("rules")]
    public async Task<ActionResult<List<AutoTaggingRuleResource>>> GetAllRules(CancellationToken ct = default)
    {
        var rules = await _ruleRepository.AllAsync(ct).ConfigureAwait(false);
        return Ok(rules.Select(ToResource).ToList());
    }

    [HttpGet("rules/{id:int}")]
    public async Task<ActionResult<AutoTaggingRuleResource>> GetRuleById(int id, CancellationToken ct = default)
    {
        var rule = await _ruleRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (rule == null)
        {
            return NotFound(new { error = $"Rule {id} not found" });
        }

        return Ok(ToResource(rule));
    }

    [HttpPost("rules")]
    public async Task<ActionResult<AutoTaggingRuleResource>> CreateRule(
        [FromBody][Required] AutoTaggingRuleResource resource,
        CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(resource.Name))
        {
            return BadRequest(new { error = "Name is required" });
        }

        if (string.IsNullOrWhiteSpace(resource.ConditionValue))
        {
            return BadRequest(new { error = "ConditionValue is required" });
        }

        if (resource.TagId <= 0)
        {
            return BadRequest(new { error = "TagId must be a positive integer" });
        }

        var rule = ToModel(resource);
        var created = await _ruleRepository.InsertAsync(rule, ct).ConfigureAwait(false);
        return CreatedAtAction(nameof(GetRuleById), new { id = created.Id }, ToResource(created));
    }

    [HttpPut("rules/{id:int}")]
    public async Task<ActionResult<AutoTaggingRuleResource>> UpdateRule(
        int id,
        [FromBody][Required] AutoTaggingRuleResource resource,
        CancellationToken ct = default)
    {
        var existing = await _ruleRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (existing == null)
        {
            return NotFound(new { error = $"Rule {id} not found" });
        }

        existing.Name = resource.Name;
        existing.Enabled = resource.Enabled;
        existing.ConditionType = resource.ConditionType;
        existing.ConditionValue = resource.ConditionValue;
        existing.TagId = resource.TagId;
        existing.MediaTypeFilter = resource.MediaTypeFilter;

        var updated = await _ruleRepository.UpdateAsync(existing, ct).ConfigureAwait(false);
        return Ok(ToResource(updated));
    }

    [HttpDelete("rules/{id:int}")]
    public async Task<IActionResult> DeleteRule(int id, CancellationToken ct = default)
    {
        var existing = await _ruleRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (existing == null)
        {
            return NotFound(new { error = $"Rule {id} not found" });
        }

        await _ruleRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    [HttpPost("apply")]
    public async Task<ActionResult<ApplyAutoTagsResult>> ApplyRulesToAll(
        [FromQuery] MediaType? mediaType,
        CancellationToken ct = default)
    {
        var updatedCount = await _autoTaggingService.ApplyAutoTagsToAllAsync(mediaType, ct).ConfigureAwait(false);
        return Ok(new ApplyAutoTagsResult { UpdatedItems = updatedCount });
    }

    [HttpGet("preview/{id:int}")]
    public async Task<ActionResult<PreviewTagsResult>> PreviewTags(int id, CancellationToken ct = default)
    {
        var item = await _mediaItemRepository.FindByIdAsync(id, ct).ConfigureAwait(false);
        if (item == null)
        {
            return NotFound(new { error = $"Media item {id} not found" });
        }

        var matchingTags = await _autoTaggingService.PreviewTagsAsync(item, ct).ConfigureAwait(false);
        return Ok(new PreviewTagsResult
        {
            MediaItemId = id,
            MatchingTagIds = matchingTags.ToList()
        });
    }

    private static AutoTaggingRuleResource ToResource(AutoTaggingRule rule)
    {
        return new AutoTaggingRuleResource
        {
            Id = rule.Id,
            Name = rule.Name,
            Enabled = rule.Enabled,
            ConditionType = rule.ConditionType,
            ConditionValue = rule.ConditionValue,
            TagId = rule.TagId,
            MediaTypeFilter = rule.MediaTypeFilter
        };
    }

    private static AutoTaggingRule ToModel(AutoTaggingRuleResource resource)
    {
        return new AutoTaggingRule
        {
            Id = resource.Id,
            Name = resource.Name,
            Enabled = resource.Enabled,
            ConditionType = resource.ConditionType,
            ConditionValue = resource.ConditionValue,
            TagId = resource.TagId,
            MediaTypeFilter = resource.MediaTypeFilter
        };
    }
}

public class AutoTaggingRuleResource
{
    public int Id { get; set; }
    public string Name { get; set; } = string.Empty;
    public bool Enabled { get; set; } = true;
    public AutoTaggingConditionType ConditionType { get; set; }
    public string ConditionValue { get; set; } = string.Empty;
    public int TagId { get; set; }
    public MediaType? MediaTypeFilter { get; set; }
}

public class ApplyAutoTagsResult
{
    public int UpdatedItems { get; set; }
}

public class PreviewTagsResult
{
    public int MediaItemId { get; set; }
    public List<int> MatchingTagIds { get; set; } = new();
}
