// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.History;

namespace Mouseion.Api.History;

[ApiController]
[Route("api/v3/history")]
[Authorize]
public class HistoryController : ControllerBase
{
    private readonly IMediaItemHistoryService _historyService;

    public HistoryController(IMediaItemHistoryService historyService)
    {
        _historyService = historyService;
    }

    [HttpGet]
    public async Task<ActionResult<PagedHistoryResource>> GetAll(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 10,
        [FromQuery] string? sortKey = null,
        [FromQuery] string? sortDirection = null,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1 || pageSize > 100) pageSize = 10;

        var (records, totalRecords) = await _historyService.GetPagedAsync(page, pageSize, sortKey, sortDirection, ct);

        return Ok(new PagedHistoryResource
        {
            Page = page,
            PageSize = pageSize,
            SortKey = sortKey,
            SortDirection = sortDirection,
            TotalRecords = totalRecords,
            Records = records.Select(h => h.ToResource()).ToList()
        });
    }

    [HttpGet("mediaitem/{mediaItemId:int}")]
    public async Task<ActionResult<List<HistoryResource>>> GetByMediaItem(int mediaItemId, CancellationToken ct)
    {
        var history = await _historyService.GetByMediaItemIdAsync(mediaItemId, ct);
        return Ok(history.Select(h => h.ToResource()).ToList());
    }

    [HttpGet("since")]
    public async Task<ActionResult<List<HistoryResource>>> GetSince([FromQuery] DateTime date, CancellationToken ct)
    {
        var history = await _historyService.SinceAsync(date, ct);
        return Ok(history.Select(h => h.ToResource()).ToList());
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<HistoryResource>> GetById(int id, CancellationToken ct)
    {
        var history = await _historyService.GetAllAsync(ct);
        var record = history.FirstOrDefault(h => h.Id == id);

        if (record == null)
        {
            return NotFound(new { message = $"History record with ID {id} not found" });
        }

        return Ok(record.ToResource());
    }

    [HttpPost]
    public async Task<ActionResult<HistoryResource>> Create([FromBody] HistoryResource resource, CancellationToken ct)
    {
        var history = resource.ToModel();
        await _historyService.AddAsync(history, ct);
        return CreatedAtAction(nameof(GetById), new { id = history.Id }, history.ToResource());
    }

    [HttpDelete("{id:int}")]
    public async Task<ActionResult> Delete(int id, CancellationToken ct)
    {
        await _historyService.DeleteAsync(id, ct);
        return NoContent();
    }

    [HttpPost("purge")]
    public async Task<ActionResult> Purge([FromQuery] DateTime olderThan, CancellationToken ct)
    {
        await _historyService.PurgeOlderThanAsync(olderThan, ct);
        return NoContent();
    }
}
