// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.RegularExpressions;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Tags;

namespace Mouseion.Api.Tags;

[ApiController]
[Route("api/v3/tags")]
[Authorize]
public class TagController : ControllerBase
{
    private readonly ITagService _tagService;

    public TagController(ITagService tagService)
    {
        _tagService = tagService;
    }

    [HttpGet]
    public async Task<ActionResult<List<TagResource>>> GetAll(CancellationToken ct = default)
    {
        var tags = await _tagService.AllAsync(ct).ConfigureAwait(false);
        return Ok(tags.Select(ToResource).ToList());
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<TagResource>> GetById(int id, CancellationToken ct = default)
    {
        try
        {
            var tag = await _tagService.GetTagAsync(id, ct).ConfigureAwait(false);
            return Ok(ToResource(tag));
        }
        catch (InvalidOperationException)
        {
            return NotFound(new { error = $"Tag {id} not found" });
        }
    }

    [HttpPost]
    public async Task<ActionResult<TagResource>> Create([FromBody][Required] TagResource resource, CancellationToken ct = default)
    {
        if (!IsValidLabel(resource.Label))
        {
            return BadRequest(new { error = "Label must contain only lowercase letters, numbers, and hyphens" });
        }

        var tag = ToModel(resource);
        var created = await _tagService.AddAsync(tag, ct).ConfigureAwait(false);
        return CreatedAtAction(nameof(GetById), new { id = created.Id }, ToResource(created));
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<TagResource>> Update(int id, [FromBody][Required] TagResource resource, CancellationToken ct = default)
    {
        if (!IsValidLabel(resource.Label))
        {
            return BadRequest(new { error = "Label must contain only lowercase letters, numbers, and hyphens" });
        }

        try
        {
            var tag = await _tagService.GetTagAsync(id, ct).ConfigureAwait(false);
            tag.Label = resource.Label;
            var updated = await _tagService.UpdateAsync(tag, ct).ConfigureAwait(false);
            return Ok(ToResource(updated));
        }
        catch (InvalidOperationException)
        {
            return NotFound(new { error = $"Tag {id} not found" });
        }
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> Delete(int id, CancellationToken ct = default)
    {
        try
        {
            await _tagService.DeleteAsync(id, ct).ConfigureAwait(false);
            return NoContent();
        }
        catch (InvalidOperationException)
        {
            return NotFound(new { error = $"Tag {id} not found" });
        }
    }

    private static bool IsValidLabel(string label)
    {
        if (string.IsNullOrWhiteSpace(label))
        {
            return false;
        }

        return Regex.IsMatch(label, "^[a-z0-9-]+$");
    }

    private static TagResource ToResource(Tag tag)
    {
        return new TagResource
        {
            Id = tag.Id,
            Label = tag.Label
        };
    }

    private static Tag ToModel(TagResource resource)
    {
        return new Tag
        {
            Id = resource.Id,
            Label = resource.Label
        };
    }
}

public class TagResource
{
    public int Id { get; set; }
    public string Label { get; set; } = null!;
}
