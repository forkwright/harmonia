// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.Manga;

namespace Mouseion.Api.Manga;

[ApiController]
[Route("api/v3/manga")]
[Authorize]
public class MangaSeriesController : ControllerBase
{
    private readonly IMangaSeriesRepository _seriesRepository;
    private readonly IMangaChapterRepository _chapterRepository;
    private readonly IAddMangaSeriesService _addSeriesService;
    private readonly IRefreshMangaSeriesService _refreshService;

    public MangaSeriesController(
        IMangaSeriesRepository seriesRepository,
        IMangaChapterRepository chapterRepository,
        IAddMangaSeriesService addSeriesService,
        IRefreshMangaSeriesService refreshService)
    {
        _seriesRepository = seriesRepository;
        _chapterRepository = chapterRepository;
        _addSeriesService = addSeriesService;
        _refreshService = refreshService;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<MangaSeriesResource>>> GetSeries(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var totalCount = await _seriesRepository.CountAsync(ct).ConfigureAwait(false);
        var series = await _seriesRepository.GetPageAsync(page, pageSize, ct).ConfigureAwait(false);

        var resources = new List<MangaSeriesResource>();
        foreach (var s in series)
        {
            var resource = ToSeriesResource(s);
            resource.UnreadChapterCount = await _chapterRepository.GetUnreadCountBySeriesAsync(s.Id, ct).ConfigureAwait(false);
            resources.Add(resource);
        }

        return Ok(new PagedResult<MangaSeriesResource>
        {
            Items = resources,
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<MangaSeriesResource>> GetSeriesById(int id, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Manga series {id} not found" });
        }

        var resource = ToSeriesResource(series);
        resource.UnreadChapterCount = await _chapterRepository.GetUnreadCountBySeriesAsync(id, ct).ConfigureAwait(false);
        return Ok(resource);
    }

    [HttpGet("{id:int}/chapters")]
    public async Task<ActionResult<PagedResult<MangaChapterResource>>> GetSeriesChapters(
        int id,
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        [FromQuery] bool? unreadOnly = null,
        CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Manga series {id} not found" });
        }

        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var chapters = await _chapterRepository.GetBySeriesIdAsync(id, ct).ConfigureAwait(false);
        if (unreadOnly == true)
        {
            chapters = chapters.Where(c => !c.IsRead).ToList();
        }

        var totalCount = chapters.Count;
        var pagedChapters = chapters.Skip((page - 1) * pageSize).Take(pageSize);

        return Ok(new PagedResult<MangaChapterResource>
        {
            Items = pagedChapters.Select(ToChapterResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("search")]
    public async Task<ActionResult<List<MangaSeriesResource>>> Search(
        [FromQuery] string q,
        CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(q))
        {
            return BadRequest(new { error = "Search query is required" });
        }

        var results = await _addSeriesService.SearchAsync(q, ct).ConfigureAwait(false);
        return Ok(results.Select(ToSeriesResource));
    }

    [HttpPost]
    public async Task<ActionResult<MangaSeriesResource>> AddSeries(
        [FromBody][Required] AddMangaSeriesRequest request,
        CancellationToken ct = default)
    {
        MangaSeries series;

        if (!string.IsNullOrEmpty(request.MangaDexId))
        {
            series = await _addSeriesService.AddByMangaDexIdAsync(
                request.MangaDexId,
                request.RootFolderPath,
                request.QualityProfileId,
                request.Monitored,
                ct).ConfigureAwait(false);
        }
        else if (request.AniListId.HasValue)
        {
            series = await _addSeriesService.AddByAniListIdAsync(
                request.AniListId.Value,
                request.RootFolderPath,
                request.QualityProfileId,
                request.Monitored,
                ct).ConfigureAwait(false);
        }
        else
        {
            return BadRequest(new { error = "Either MangaDexId or AniListId is required" });
        }

        return CreatedAtAction(nameof(GetSeriesById), new { id = series.Id }, ToSeriesResource(series));
    }

    [HttpPost("{id:int}/refresh")]
    public async Task<ActionResult<RefreshResult>> RefreshSeries(int id, CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Manga series {id} not found" });
        }

        var newChapterCount = await _refreshService.RefreshSeriesAsync(id, ct).ConfigureAwait(false);
        return Ok(new RefreshResult { NewChapterCount = newChapterCount });
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<MangaSeriesResource>> UpdateSeries(
        int id,
        [FromBody][Required] MangaSeriesResource resource,
        CancellationToken ct = default)
    {
        var series = await _seriesRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (series == null)
        {
            return NotFound(new { error = $"Manga series {id} not found" });
        }

        series.Title = resource.Title;
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
            return NotFound(new { error = $"Manga series {id} not found" });
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
            return NotFound(new { error = $"Manga series {id} not found" });
        }

        await _chapterRepository.MarkAllReadBySeriesAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private static MangaSeriesResource ToSeriesResource(MangaSeries series)
    {
        return new MangaSeriesResource
        {
            Id = series.Id,
            Title = series.Title,
            SortTitle = series.SortTitle,
            Description = series.Description,
            MangaDexId = series.MangaDexId,
            AniListId = series.AniListId,
            MyAnimeListId = series.MyAnimeListId,
            Author = series.Author,
            Artist = series.Artist,
            Status = series.Status,
            Year = series.Year,
            OriginalLanguage = series.OriginalLanguage,
            ContentRating = series.ContentRating,
            Genres = series.Genres,
            Tags = series.Tags,
            CoverUrl = series.CoverUrl,
            LastChapterNumber = series.LastChapterNumber,
            LastVolumeNumber = series.LastVolumeNumber,
            ChapterCount = series.ChapterCount,
            Monitored = series.Monitored,
            Path = series.Path,
            RootFolderPath = series.RootFolderPath,
            QualityProfileId = series.QualityProfileId,
            Added = series.Added
        };
    }

    private static MangaChapterResource ToChapterResource(MangaChapter chapter)
    {
        return new MangaChapterResource
        {
            Id = chapter.Id,
            MangaSeriesId = chapter.MangaSeriesId,
            Title = chapter.Title,
            ChapterNumber = chapter.ChapterNumber,
            VolumeNumber = chapter.VolumeNumber,
            MangaDexChapterId = chapter.MangaDexChapterId,
            ScanlationGroup = chapter.ScanlationGroup,
            TranslatedLanguage = chapter.TranslatedLanguage,
            PageCount = chapter.PageCount,
            ExternalUrl = chapter.ExternalUrl,
            PublishDate = chapter.PublishDate,
            IsRead = chapter.IsRead,
            IsDownloaded = chapter.IsDownloaded,
            Added = chapter.Added
        };
    }
}

public class MangaSeriesResource
{
    public int Id { get; set; }
    public string Title { get; set; } = string.Empty;
    public string? SortTitle { get; set; }
    public string? Description { get; set; }
    public string? MangaDexId { get; set; }
    public int? AniListId { get; set; }
    public int? MyAnimeListId { get; set; }
    public string? Author { get; set; }
    public string? Artist { get; set; }
    public string? Status { get; set; }
    public int? Year { get; set; }
    public string? OriginalLanguage { get; set; }
    public string? ContentRating { get; set; }
    public string? Genres { get; set; }
    public string? Tags { get; set; }
    public string? CoverUrl { get; set; }
    public decimal? LastChapterNumber { get; set; }
    public int? LastVolumeNumber { get; set; }
    public int? ChapterCount { get; set; }
    public int UnreadChapterCount { get; set; }
    public bool Monitored { get; set; }
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; }
    public DateTime Added { get; set; }
}

public class MangaChapterResource
{
    public int Id { get; set; }
    public int MangaSeriesId { get; set; }
    public string? Title { get; set; }
    public decimal? ChapterNumber { get; set; }
    public int? VolumeNumber { get; set; }
    public string? MangaDexChapterId { get; set; }
    public string? ScanlationGroup { get; set; }
    public string? TranslatedLanguage { get; set; }
    public int? PageCount { get; set; }
    public string? ExternalUrl { get; set; }
    public DateTime? PublishDate { get; set; }
    public bool IsRead { get; set; }
    public bool IsDownloaded { get; set; }
    public DateTime Added { get; set; }
}

public class AddMangaSeriesRequest
{
    public string? MangaDexId { get; set; }
    public int? AniListId { get; set; }
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; } = 1;
    public bool Monitored { get; set; } = true;
}

public class RefreshResult
{
    public int NewChapterCount { get; set; }
}
