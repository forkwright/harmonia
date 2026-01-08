// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Blocklisting;

namespace Mouseion.Api.Blocklisting;

[ApiController]
[Route("api/v3/blocklist")]
[Authorize]
public class BlocklistController : ControllerBase
{
    private readonly IBlocklistService _blocklistService;

    public BlocklistController(IBlocklistService blocklistService)
    {
        _blocklistService = blocklistService;
    }

    [HttpGet]
    public async Task<ActionResult<List<BlocklistResource>>> GetAll([FromQuery] int? mediaItemId, CancellationToken ct)
    {
        if (mediaItemId.HasValue)
        {
            var blocklist = await _blocklistService.GetByMediaItemIdAsync(mediaItemId.Value, ct);
            return Ok(blocklist.Select(b => b.ToResource()).ToList());
        }

        var all = await _blocklistService.GetAllAsync(ct);
        return Ok(all.Select(b => b.ToResource()).ToList());
    }

    [HttpGet("mediaitem/{mediaItemId:int}")]
    public async Task<ActionResult<List<BlocklistResource>>> GetByMediaItem(int mediaItemId, CancellationToken ct)
    {
        var blocklist = await _blocklistService.GetByMediaItemIdAsync(mediaItemId, ct);
        return Ok(blocklist.Select(b => b.ToResource()).ToList());
    }

    [HttpPost]
    public async Task<ActionResult<BlocklistResource>> Create([FromBody][Required] BlocklistResource resource, CancellationToken ct)
    {
        var blocklist = resource.ToModel();
        await _blocklistService.AddAsync(blocklist, ct);
        return CreatedAtAction(nameof(GetByMediaItem), new { mediaItemId = blocklist.MediaItemId }, blocklist.ToResource());
    }

    [HttpDelete("{id:int}")]
    public async Task<ActionResult> Delete(int id, CancellationToken ct)
    {
        await _blocklistService.DeleteAsync(id, ct);
        return NoContent();
    }

    [HttpDelete("bulk")]
    public async Task<ActionResult> DeleteBulk([FromBody][Required] List<int> ids, CancellationToken ct)
    {
        await _blocklistService.DeleteManyAsync(ids, ct);
        return NoContent();
    }

    [HttpDelete("clear")]
    public async Task<ActionResult> ClearAll(CancellationToken ct)
    {
        await _blocklistService.ClearAllAsync(ct);
        return NoContent();
    }
}
