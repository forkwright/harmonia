// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.Comic;
using Mouseion.Core.Comic.ComicVine;

namespace Mouseion.Api.Comic;

[ApiController]
[Route("api/v3/comic")]
[Authorize]
public class ComicSeriesController : ControllerBase
{
    private readonly IComicSeriesRepository _seriesRepository;
    private readonly IComicIssueRepository _issueRepository;
    private readonly IAddComicSeriesService _addSeriesService;
    private readonly IRefreshComicSeriesService _refreshService;

    public ComicSeriesController(
        IComicSeriesRepository seriesRepository,
        IComicIssueRepository issueRepository,
        IAddComicSeriesService addSeriesService,
        IRefreshComicSeriesService refreshService)
    {
        _seriesRepository = seriesRepository;
        _issueRepository = issueRepository;
        _addSeriesService = addSeriesService;
        _refreshService = refreshService;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<ComicSeriesResource>>> GetSeries(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var totalCount = await _seriesRepository.CountAsync(ct).ConfigureAwait(false);
        var series = await _seriesRepository.GetPageAsync(page, pageSize, ct).ConfigureAwait(false);

        var resources = new List<ComicSeriesResource>();
        foreach (var s in series)
        {
            var resource = ToSeriesResource(s);
            resource.UnreadIssueCount = await _issueRepository.GetUnreadCountBySeriesAsync(s.Id, ct).ConfigureAwait(false);
            resources.Add(resource);
        }

        return Ok(new PagedResult<ComicSeriesResource>
        {
            Items = resources,
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<ComicSeriesResource>> GetSeriesById(int id, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Comic series {id} not found" });
        }

        var resource = ToSeriesResource(series);
        resource.UnreadIssueCount = await _issueRepository.GetUnreadCountBySeriesAsync(id, ct).ConfigureAwait(false);
        return Ok(resource);
    }

    [HttpGet("{id:int}/issues")]
    public async Task<ActionResult<PagedResult<ComicIssueResource>>> GetSeriesIssues(
        int id,
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        [FromQuery] bool? unreadOnly = null,
        CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Comic series {id} not found" });
        }

        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var issues = await _issueRepository.GetBySeriesIdAsync(id, ct).ConfigureAwait(false);
        if (unreadOnly == true)
        {
            issues = issues.Where(i => !i.IsRead).ToList();
        }

        var totalCount = issues.Count;
        var pagedIssues = issues.Skip((page - 1) * pageSize).Take(pageSize);

        return Ok(new PagedResult<ComicIssueResource>
        {
            Items = pagedIssues.Select(ToIssueResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("search")]
    public async Task<ActionResult<List<ComicVineVolume>>> Search(
        [FromQuery][Required] string query,
        [FromQuery] int limit = 10,
        CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(query))
        {
            return BadRequest(new { error = "Query parameter is required" });
        }

        var results = await _addSeriesService.SearchAsync(query, limit, ct).ConfigureAwait(false);
        return Ok(results);
    }

    [HttpPost]
    public async Task<ActionResult<ComicSeriesResource>> AddSeries(
        [FromBody][Required] AddComicSeriesRequest request,
        CancellationToken ct = default)
    {
        if (!request.ComicVineId.HasValue)
        {
            return BadRequest(new { error = "ComicVineId is required" });
        }

        var series = await _addSeriesService.AddByComicVineIdAsync(
            request.ComicVineId.Value,
            request.RootFolderPath,
            request.QualityProfileId,
            ct).ConfigureAwait(false);

        if (series == null)
        {
            return BadRequest(new { error = "Failed to add comic series" });
        }

        return CreatedAtAction(nameof(GetSeriesById), new { id = series.Id }, ToSeriesResource(series));
    }

    [HttpPost("{id:int}/refresh")]
    public async Task<ActionResult<RefreshResult>> RefreshSeries(int id, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Comic series {id} not found" });
        }

        var newIssues = await _refreshService.RefreshSeriesAsync(id, ct).ConfigureAwait(false);
        return Ok(new RefreshResult { NewIssuesCount = newIssues });
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<ComicSeriesResource>> UpdateSeries(
        int id,
        [FromBody][Required] ComicSeriesResource resource,
        CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Comic series {id} not found" });
        }

        series.Title = resource.Title;
        series.Description = resource.Description;
        series.Monitored = resource.Monitored;
        series.QualityProfileId = resource.QualityProfileId;
        series.Path = resource.Path;
        series.RootFolderPath = resource.RootFolderPath;

        var updated = await _seriesRepository.UpdateAsync(series, ct).ConfigureAwait(false);
        return Ok(ToSeriesResource(updated));
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeleteSeries(int id, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Comic series {id} not found" });
        }

        await _seriesRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    [HttpPost("{id:int}/markallread")]
    public async Task<IActionResult> MarkAllRead(int id, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Comic series {id} not found" });
        }

        await _issueRepository.MarkAllReadBySeriesAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private static ComicSeriesResource ToSeriesResource(ComicSeries series)
    {
        return new ComicSeriesResource
        {
            Id = series.Id,
            Title = series.Title,
            SortTitle = series.SortTitle,
            Description = series.Description,
            ComicVineId = series.ComicVineId,
            Publisher = series.Publisher,
            Imprint = series.Imprint,
            StartYear = series.StartYear,
            EndYear = series.EndYear,
            Status = series.Status,
            IssueCount = series.IssueCount,
            VolumeNumber = series.VolumeNumber,
            Genres = series.Genres,
            Characters = series.Characters,
            CoverUrl = series.CoverUrl,
            SiteUrl = series.SiteUrl,
            Monitored = series.Monitored,
            Path = series.Path,
            RootFolderPath = series.RootFolderPath,
            QualityProfileId = series.QualityProfileId,
            Added = series.Added
        };
    }

    private static ComicIssueResource ToIssueResource(ComicIssue issue)
    {
        return new ComicIssueResource
        {
            Id = issue.Id,
            ComicSeriesId = issue.ComicSeriesId,
            Title = issue.Title,
            IssueNumber = issue.IssueNumber,
            ComicVineIssueId = issue.ComicVineIssueId,
            StoryArc = issue.StoryArc,
            Writer = issue.Writer,
            Penciler = issue.Penciler,
            Inker = issue.Inker,
            Colorist = issue.Colorist,
            CoverArtist = issue.CoverArtist,
            CoverDate = issue.CoverDate,
            StoreDate = issue.StoreDate,
            PageCount = issue.PageCount,
            CoverUrl = issue.CoverUrl,
            SiteUrl = issue.SiteUrl,
            Description = issue.Description,
            IsRead = issue.IsRead,
            IsDownloaded = issue.IsDownloaded,
            FileFormat = issue.FileFormat,
            Added = issue.Added
        };
    }
}

public class RefreshResult
{
    public int NewIssuesCount { get; set; }
}

public class ComicSeriesResource
{
    public int Id { get; set; }
    public string Title { get; set; } = string.Empty;
    public string? SortTitle { get; set; }
    public string? Description { get; set; }
    public int? ComicVineId { get; set; }
    public string? Publisher { get; set; }
    public string? Imprint { get; set; }
    public int? StartYear { get; set; }
    public int? EndYear { get; set; }
    public string? Status { get; set; }
    public int? IssueCount { get; set; }
    public int? VolumeNumber { get; set; }
    public string? Genres { get; set; }
    public string? Characters { get; set; }
    public string? CoverUrl { get; set; }
    public string? SiteUrl { get; set; }
    public int UnreadIssueCount { get; set; }
    public bool Monitored { get; set; }
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; }
    public DateTime Added { get; set; }
}

public class ComicIssueResource
{
    public int Id { get; set; }
    public int ComicSeriesId { get; set; }
    public string? Title { get; set; }
    public string? IssueNumber { get; set; }
    public int? ComicVineIssueId { get; set; }
    public string? StoryArc { get; set; }
    public string? Writer { get; set; }
    public string? Penciler { get; set; }
    public string? Inker { get; set; }
    public string? Colorist { get; set; }
    public string? CoverArtist { get; set; }
    public DateTime? CoverDate { get; set; }
    public DateTime? StoreDate { get; set; }
    public int? PageCount { get; set; }
    public string? CoverUrl { get; set; }
    public string? SiteUrl { get; set; }
    public string? Description { get; set; }
    public bool IsRead { get; set; }
    public bool IsDownloaded { get; set; }
    public string? FileFormat { get; set; }
    public DateTime Added { get; set; }
}

public class AddComicSeriesRequest
{
    public int? ComicVineId { get; set; }
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; } = 1;
    public bool Monitored { get; set; } = true;
}
