// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.Webcomic;

namespace Mouseion.Api.Webcomic;

[ApiController]
[Route("api/v3/webcomic/episodes")]
[Authorize]
public class WebcomicEpisodesController : ControllerBase
{
    private readonly IWebcomicEpisodeRepository _episodeRepository;

    public WebcomicEpisodesController(IWebcomicEpisodeRepository episodeRepository)
    {
        _episodeRepository = episodeRepository;
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<WebcomicEpisodeResource>> GetEpisode(int id, CancellationToken ct = default)
    {
        var episode = await _episodeRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (episode == null)
        {
            return NotFound(new { error = $"Episode {id} not found" });
        }

        return Ok(ToEpisodeResource(episode));
    }

    [HttpGet("unread")]
    public async Task<ActionResult<PagedResult<WebcomicEpisodeResource>>> GetUnread(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var unreadEpisodes = await _episodeRepository.GetUnreadAsync(ct).ConfigureAwait(false);
        var totalCount = unreadEpisodes.Count;
        var pagedEpisodes = unreadEpisodes.Skip((page - 1) * pageSize).Take(pageSize);

        return Ok(new PagedResult<WebcomicEpisodeResource>
        {
            Items = pagedEpisodes.Select(ToEpisodeResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("unread/count")]
    public async Task<ActionResult<UnreadCountResult>> GetUnreadCount(CancellationToken ct = default)
    {
        var count = await _episodeRepository.GetUnreadCountAsync(ct).ConfigureAwait(false);
        return Ok(new UnreadCountResult { Count = count });
    }

    [HttpPut("{id:int}/read")]
    public async Task<IActionResult> MarkRead(int id, CancellationToken ct = default)
    {
        var episode = await _episodeRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (episode == null)
        {
            return NotFound(new { error = $"Episode {id} not found" });
        }

        await _episodeRepository.MarkReadAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    [HttpPut("{id:int}/unread")]
    public async Task<IActionResult> MarkUnread(int id, CancellationToken ct = default)
    {
        var episode = await _episodeRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (episode == null)
        {
            return NotFound(new { error = $"Episode {id} not found" });
        }

        await _episodeRepository.MarkUnreadAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private static WebcomicEpisodeResource ToEpisodeResource(WebcomicEpisode episode)
    {
        return new WebcomicEpisodeResource
        {
            Id = episode.Id,
            WebcomicSeriesId = episode.WebcomicSeriesId,
            Title = episode.Title,
            EpisodeNumber = episode.EpisodeNumber,
            SeasonNumber = episode.SeasonNumber,
            ExternalId = episode.ExternalId,
            ExternalUrl = episode.ExternalUrl,
            ThumbnailUrl = episode.ThumbnailUrl,
            PublishDate = episode.PublishDate,
            IsRead = episode.IsRead,
            IsDownloaded = episode.IsDownloaded,
            Added = episode.Added
        };
    }
}

public class UnreadCountResult
{
    public int Count { get; set; }
}
