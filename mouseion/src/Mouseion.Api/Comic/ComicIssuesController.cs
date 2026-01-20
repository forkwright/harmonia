// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.Comic;

namespace Mouseion.Api.Comic;

[ApiController]
[Route("api/v3/comic/issues")]
[Authorize]
public class ComicIssuesController : ControllerBase
{
    private readonly IComicIssueRepository _issueRepository;

    public ComicIssuesController(IComicIssueRepository issueRepository)
    {
        _issueRepository = issueRepository;
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<ComicIssueResource>> GetIssue(int id, CancellationToken ct = default)
    {
        var issue = await _issueRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (issue == null)
        {
            return NotFound(new { error = $"Issue {id} not found" });
        }

        return Ok(ToIssueResource(issue));
    }

    [HttpGet("unread")]
    public async Task<ActionResult<PagedResult<ComicIssueResource>>> GetUnread(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var unreadIssues = await _issueRepository.GetUnreadAsync(ct).ConfigureAwait(false);
        var totalCount = unreadIssues.Count;
        var pagedIssues = unreadIssues.Skip((page - 1) * pageSize).Take(pageSize);

        return Ok(new PagedResult<ComicIssueResource>
        {
            Items = pagedIssues.Select(ToIssueResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("unread/count")]
    public async Task<ActionResult<UnreadCountResult>> GetUnreadCount(CancellationToken ct = default)
    {
        var count = await _issueRepository.GetUnreadCountAsync(ct).ConfigureAwait(false);
        return Ok(new UnreadCountResult { Count = count });
    }

    [HttpPut("{id:int}/read")]
    public async Task<IActionResult> MarkRead(int id, CancellationToken ct = default)
    {
        var issue = await _issueRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (issue == null)
        {
            return NotFound(new { error = $"Issue {id} not found" });
        }

        await _issueRepository.MarkReadAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    [HttpPut("{id:int}/unread")]
    public async Task<IActionResult> MarkUnread(int id, CancellationToken ct = default)
    {
        var issue = await _issueRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (issue == null)
        {
            return NotFound(new { error = $"Issue {id} not found" });
        }

        await _issueRepository.MarkUnreadAsync(id, ct).ConfigureAwait(false);
        return NoContent();
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

public class UnreadCountResult
{
    public int Count { get; set; }
}
