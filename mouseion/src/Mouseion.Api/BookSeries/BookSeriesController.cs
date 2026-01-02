// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.BookSeries;

namespace Mouseion.Api.BookSeries;

[ApiController]
[Route("api/v3/series")]
[Authorize]
public class BookSeriesController : ControllerBase
{
    private readonly IBookSeriesRepository _seriesRepository;

    public BookSeriesController(IBookSeriesRepository seriesRepository)
    {
        _seriesRepository = seriesRepository;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<BookSeriesResource>>> GetAllSeries(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var totalCount = await _seriesRepository.CountAsync(ct).ConfigureAwait(false);
        var series = await _seriesRepository.GetPageAsync(page, pageSize, ct).ConfigureAwait(false);

        return Ok(new PagedResult<BookSeriesResource>
        {
            Items = series.Select(ToResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<BookSeriesResource>> GetSeries(int id, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Series {id} not found" });
        }

        return Ok(ToResource(series));
    }

    [HttpGet("author/{authorId:int}")]
    public async Task<ActionResult<List<BookSeriesResource>>> GetSeriesByAuthor(int authorId, CancellationToken ct = default)
    {
        var series = await _seriesRepository.GetByAuthorIdAsync(authorId, ct).ConfigureAwait(false);
        return Ok(series.Select(ToResource).ToList());
    }

    [HttpGet("foreignId/{foreignId}")]
    public async Task<ActionResult<BookSeriesResource>> GetByForeignId(string foreignId, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindByForeignIdAsync(foreignId, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Series with foreign ID {foreignId} not found" });
        }

        return Ok(ToResource(series));
    }

    [HttpPost]
    public async Task<ActionResult<BookSeriesResource>> AddSeries([FromBody] BookSeriesResource resource, CancellationToken ct = default)
    {
        try
        {
            var series = ToModel(resource);
            var added = await _seriesRepository.InsertAsync(series, ct).ConfigureAwait(false);
            return CreatedAtAction(nameof(GetSeries), new { id = added.Id }, ToResource(added));
        }
        catch (ArgumentException ex)
        {
            return BadRequest(new { error = ex.Message });
        }
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<BookSeriesResource>> UpdateSeries(int id, [FromBody] BookSeriesResource resource, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Series {id} not found" });
        }

        series.Title = resource.Title;
        series.SortTitle = resource.SortTitle;
        series.Description = resource.Description;
        series.ForeignSeriesId = resource.ForeignSeriesId;
        series.AuthorId = resource.AuthorId;
        series.Monitored = resource.Monitored;

        var updated = await _seriesRepository.UpdateAsync(series, ct).ConfigureAwait(false);
        return Ok(ToResource(updated));
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeleteSeries(int id, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Series {id} not found" });
        }

        await _seriesRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private static BookSeriesResource ToResource(Core.BookSeries.BookSeries series)
    {
        return new BookSeriesResource
        {
            Id = series.Id,
            Title = series.Title,
            SortTitle = series.SortTitle,
            Description = series.Description,
            ForeignSeriesId = series.ForeignSeriesId,
            AuthorId = series.AuthorId,
            Monitored = series.Monitored
        };
    }

    private static Core.BookSeries.BookSeries ToModel(BookSeriesResource resource)
    {
        return new Core.BookSeries.BookSeries
        {
            Id = resource.Id,
            Title = resource.Title,
            SortTitle = resource.SortTitle,
            Description = resource.Description,
            ForeignSeriesId = resource.ForeignSeriesId,
            AuthorId = resource.AuthorId,
            Monitored = resource.Monitored
        };
    }
}

public class BookSeriesResource
{
    public int Id { get; set; }
    public string Title { get; set; } = null!;
    public string? SortTitle { get; set; }
    public string? Description { get; set; }
    public string? ForeignSeriesId { get; set; }
    public int? AuthorId { get; set; }
    public bool Monitored { get; set; }
}
