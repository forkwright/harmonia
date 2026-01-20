// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.Manga;

namespace Mouseion.Api.Manga;

[ApiController]
[Route("api/v3/manga/chapters")]
[Authorize]
public class MangaChaptersController : ControllerBase
{
    private readonly IMangaChapterRepository _chapterRepository;

    public MangaChaptersController(IMangaChapterRepository chapterRepository)
    {
        _chapterRepository = chapterRepository;
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<MangaChapterResource>> GetChapter(int id, CancellationToken ct = default)
    {
        var chapter = await _chapterRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (chapter == null)
        {
            return NotFound(new { error = $"Chapter {id} not found" });
        }

        return Ok(ToChapterResource(chapter));
    }

    [HttpGet("unread")]
    public async Task<ActionResult<PagedResult<MangaChapterResource>>> GetUnread(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var unreadChapters = await _chapterRepository.GetUnreadAsync(ct).ConfigureAwait(false);
        var totalCount = unreadChapters.Count;
        var pagedChapters = unreadChapters.Skip((page - 1) * pageSize).Take(pageSize);

        return Ok(new PagedResult<MangaChapterResource>
        {
            Items = pagedChapters.Select(ToChapterResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("unread/count")]
    public async Task<ActionResult<UnreadCountResult>> GetUnreadCount(CancellationToken ct = default)
    {
        var count = await _chapterRepository.GetUnreadCountAsync(ct).ConfigureAwait(false);
        return Ok(new UnreadCountResult { Count = count });
    }

    [HttpPut("{id:int}/read")]
    public async Task<IActionResult> MarkRead(int id, CancellationToken ct = default)
    {
        var chapter = await _chapterRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (chapter == null)
        {
            return NotFound(new { error = $"Chapter {id} not found" });
        }

        await _chapterRepository.MarkReadAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    [HttpPut("{id:int}/unread")]
    public async Task<IActionResult> MarkUnread(int id, CancellationToken ct = default)
    {
        var chapter = await _chapterRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (chapter == null)
        {
            return NotFound(new { error = $"Chapter {id} not found" });
        }

        await _chapterRepository.MarkUnreadAsync(id, ct).ConfigureAwait(false);
        return NoContent();
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

public class UnreadCountResult
{
    public int Count { get; set; }
}
