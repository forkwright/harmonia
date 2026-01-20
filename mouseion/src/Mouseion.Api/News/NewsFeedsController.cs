// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.ComponentModel.DataAnnotations;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.News;

namespace Mouseion.Api.News;

[ApiController]
[Route("api/v3/feeds")]
[Authorize]
public class NewsFeedsController : ControllerBase
{
    private readonly INewsFeedRepository _feedRepository;
    private readonly INewsArticleRepository _articleRepository;
    private readonly IAddNewsFeedService _addFeedService;
    private readonly IRefreshNewsFeedService _refreshService;

    public NewsFeedsController(
        INewsFeedRepository feedRepository,
        INewsArticleRepository articleRepository,
        IAddNewsFeedService addFeedService,
        IRefreshNewsFeedService refreshService)
    {
        _feedRepository = feedRepository;
        _articleRepository = articleRepository;
        _addFeedService = addFeedService;
        _refreshService = refreshService;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<NewsFeedResource>>> GetFeeds(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var totalCount = await _feedRepository.CountAsync(ct).ConfigureAwait(false);
        var feeds = await _feedRepository.GetPageAsync(page, pageSize, ct).ConfigureAwait(false);

        var resources = new List<NewsFeedResource>();
        foreach (var feed in feeds)
        {
            var resource = ToFeedResource(feed);
            resource.UnreadCount = await _articleRepository.GetUnreadCountByFeedAsync(feed.Id, ct).ConfigureAwait(false);
            resources.Add(resource);
        }

        return Ok(new PagedResult<NewsFeedResource>
        {
            Items = resources,
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<NewsFeedResource>> GetFeed(int id, CancellationToken ct = default)
    {
        var feed = await _feedRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (feed == null)
        {
            return NotFound(new { error = $"News feed {id} not found" });
        }

        var resource = ToFeedResource(feed);
        resource.UnreadCount = await _articleRepository.GetUnreadCountByFeedAsync(id, ct).ConfigureAwait(false);
        return Ok(resource);
    }

    [HttpGet("{id:int}/articles")]
    public async Task<ActionResult<PagedResult<NewsArticleResource>>> GetFeedArticles(
        int id,
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        [FromQuery] bool? unreadOnly = null,
        CancellationToken ct = default)
    {
        var feed = await _feedRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (feed == null)
        {
            return NotFound(new { error = $"News feed {id} not found" });
        }

        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var articles = await _articleRepository.GetByFeedIdAsync(id, ct).ConfigureAwait(false);
        if (unreadOnly == true)
        {
            articles = articles.Where(a => !a.IsRead).ToList();
        }

        var totalCount = articles.Count;
        var pagedArticles = articles.Skip((page - 1) * pageSize).Take(pageSize);

        return Ok(new PagedResult<NewsArticleResource>
        {
            Items = pagedArticles.Select(ToArticleResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpPost]
    public async Task<ActionResult<NewsFeedResource>> AddFeed(
        [FromBody][Required] AddNewsFeedRequest request,
        CancellationToken ct = default)
    {
        var feed = await _addFeedService.AddFeedAsync(
            request.FeedUrl,
            request.RootFolderPath,
            request.QualityProfileId,
            request.Monitored,
            ct).ConfigureAwait(false);

        return CreatedAtAction(nameof(GetFeed), new { id = feed.Id }, ToFeedResource(feed));
    }

    [HttpPost("{id:int}/refresh")]
    public async Task<ActionResult<RefreshResult>> RefreshFeed(int id, CancellationToken ct = default)
    {
        var feed = await _feedRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (feed == null)
        {
            return NotFound(new { error = $"News feed {id} not found" });
        }

        var newArticleCount = await _refreshService.RefreshFeedAsync(id, ct).ConfigureAwait(false);
        return Ok(new RefreshResult { NewArticleCount = newArticleCount });
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<NewsFeedResource>> UpdateFeed(
        int id,
        [FromBody][Required] NewsFeedResource resource,
        CancellationToken ct = default)
    {
        var feed = await _feedRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (feed == null)
        {
            return NotFound(new { error = $"News feed {id} not found" });
        }

        feed.Title = resource.Title;
        feed.Monitored = resource.Monitored;
        feed.RefreshInterval = resource.RefreshInterval;
        feed.QualityProfileId = resource.QualityProfileId;
        feed.Path = resource.Path;
        feed.RootFolderPath = resource.RootFolderPath;

        var updated = await _feedRepository.UpdateAsync(feed, ct).ConfigureAwait(false);
        return Ok(ToFeedResource(updated));
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeleteFeed(int id, CancellationToken ct = default)
    {
        var feed = await _feedRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (feed == null)
        {
            return NotFound(new { error = $"News feed {id} not found" });
        }

        await _feedRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    [HttpPost("{id:int}/markallread")]
    public async Task<IActionResult> MarkAllRead(int id, CancellationToken ct = default)
    {
        var feed = await _feedRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (feed == null)
        {
            return NotFound(new { error = $"News feed {id} not found" });
        }

        await _articleRepository.MarkAllReadByFeedAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private static NewsFeedResource ToFeedResource(NewsFeed feed)
    {
        return new NewsFeedResource
        {
            Id = feed.Id,
            Title = feed.Title,
            SortTitle = feed.SortTitle,
            Description = feed.Description,
            FeedUrl = feed.FeedUrl,
            SiteUrl = feed.SiteUrl,
            FaviconUrl = feed.FaviconUrl,
            Category = feed.Category,
            Language = feed.Language,
            LastFetchTime = feed.LastFetchTime,
            LastItemDate = feed.LastItemDate,
            ItemCount = feed.ItemCount,
            Monitored = feed.Monitored,
            RefreshInterval = feed.RefreshInterval,
            Path = feed.Path,
            RootFolderPath = feed.RootFolderPath,
            QualityProfileId = feed.QualityProfileId,
            Tags = feed.Tags,
            Added = feed.Added
        };
    }

    private static NewsArticleResource ToArticleResource(NewsArticle article)
    {
        return new NewsArticleResource
        {
            Id = article.Id,
            NewsFeedId = article.NewsFeedId,
            Title = article.Title,
            Author = article.Author,
            ArticleGuid = article.ArticleGuid,
            SourceUrl = article.SourceUrl,
            PublishDate = article.PublishDate,
            Description = article.Description,
            ImageUrl = article.ImageUrl,
            Categories = article.Categories,
            IsRead = article.IsRead,
            IsStarred = article.IsStarred,
            IsArchived = article.IsArchived,
            Added = article.Added
        };
    }
}

public class NewsFeedResource
{
    public int Id { get; set; }
    public string Title { get; set; } = string.Empty;
    public string? SortTitle { get; set; }
    public string? Description { get; set; }
    public string FeedUrl { get; set; } = string.Empty;
    public string? SiteUrl { get; set; }
    public string? FaviconUrl { get; set; }
    public string? Category { get; set; }
    public string? Language { get; set; }
    public DateTime? LastFetchTime { get; set; }
    public DateTime? LastItemDate { get; set; }
    public int? ItemCount { get; set; }
    public int UnreadCount { get; set; }
    public bool Monitored { get; set; }
    public int RefreshInterval { get; set; }
    public string? Path { get; set; }
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; }
    public string? Tags { get; set; }
    public DateTime Added { get; set; }
}

public class NewsArticleResource
{
    public int Id { get; set; }
    public int NewsFeedId { get; set; }
    public string Title { get; set; } = string.Empty;
    public string? Author { get; set; }
    public string? ArticleGuid { get; set; }
    public string? SourceUrl { get; set; }
    public DateTime? PublishDate { get; set; }
    public string? Description { get; set; }
    public string? Content { get; set; }
    public string? ImageUrl { get; set; }
    public string? Categories { get; set; }
    public bool IsRead { get; set; }
    public bool IsStarred { get; set; }
    public bool IsArchived { get; set; }
    public DateTime Added { get; set; }
}

public class AddNewsFeedRequest
{
    public string FeedUrl { get; set; } = string.Empty;
    public string? RootFolderPath { get; set; }
    public int QualityProfileId { get; set; } = 1;
    public bool Monitored { get; set; } = true;
}

public class RefreshResult
{
    public int NewArticleCount { get; set; }
}
