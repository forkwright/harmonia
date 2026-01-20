// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Mouseion.Core.News;

namespace Mouseion.Api.News;

[ApiController]
[Route("api/v3/articles")]
[Authorize]
public class NewsArticlesController : ControllerBase
{
    private readonly INewsArticleRepository _articleRepository;

    public NewsArticlesController(INewsArticleRepository articleRepository)
    {
        _articleRepository = articleRepository;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<NewsArticleResource>>> GetArticles(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        [FromQuery] bool? unreadOnly = null,
        [FromQuery] bool? starredOnly = null,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        List<NewsArticle> articles;
        if (starredOnly == true)
        {
            articles = await _articleRepository.GetStarredAsync(ct).ConfigureAwait(false);
        }
        else if (unreadOnly == true)
        {
            articles = await _articleRepository.GetUnreadAsync(ct).ConfigureAwait(false);
        }
        else
        {
            articles = await _articleRepository.GetRecentAsync(pageSize * page, ct).ConfigureAwait(false);
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

    [HttpGet("{id:int}")]
    public async Task<ActionResult<NewsArticleDetailResource>> GetArticle(int id, CancellationToken ct = default)
    {
        var article = await _articleRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (article == null)
        {
            return NotFound(new { error = $"Article {id} not found" });
        }

        return Ok(ToDetailResource(article));
    }

    [HttpGet("unread")]
    public async Task<ActionResult<UnreadCountResult>> GetUnreadCount(CancellationToken ct = default)
    {
        var count = await _articleRepository.GetUnreadCountAsync(ct).ConfigureAwait(false);
        return Ok(new UnreadCountResult { Count = count });
    }

    [HttpPut("{id:int}/read")]
    public async Task<IActionResult> MarkRead(int id, CancellationToken ct = default)
    {
        var article = await _articleRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (article == null)
        {
            return NotFound(new { error = $"Article {id} not found" });
        }

        await _articleRepository.MarkReadAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    [HttpDelete("{id:int}/read")]
    public async Task<IActionResult> MarkUnread(int id, CancellationToken ct = default)
    {
        var article = await _articleRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (article == null)
        {
            return NotFound(new { error = $"Article {id} not found" });
        }

        await _articleRepository.MarkUnreadAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    [HttpPut("{id:int}/star")]
    public async Task<IActionResult> Star(int id, CancellationToken ct = default)
    {
        var article = await _articleRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (article == null)
        {
            return NotFound(new { error = $"Article {id} not found" });
        }

        await _articleRepository.SetStarredAsync(id, true, ct).ConfigureAwait(false);
        return NoContent();
    }

    [HttpDelete("{id:int}/star")]
    public async Task<IActionResult> Unstar(int id, CancellationToken ct = default)
    {
        var article = await _articleRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (article == null)
        {
            return NotFound(new { error = $"Article {id} not found" });
        }

        await _articleRepository.SetStarredAsync(id, false, ct).ConfigureAwait(false);
        return NoContent();
    }

    [HttpPost("{id:int}/archive")]
    public async Task<IActionResult> Archive(int id, CancellationToken ct = default)
    {
        var article = await _articleRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (article == null)
        {
            return NotFound(new { error = $"Article {id} not found" });
        }

        article.IsArchived = true;
        article.ArchivedContent = article.Content;
        await _articleRepository.UpdateAsync(article, ct).ConfigureAwait(false);
        return NoContent();
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeleteArticle(int id, CancellationToken ct = default)
    {
        var article = await _articleRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (article == null)
        {
            return NotFound(new { error = $"Article {id} not found" });
        }

        await _articleRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
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

    private static NewsArticleDetailResource ToDetailResource(NewsArticle article)
    {
        return new NewsArticleDetailResource
        {
            Id = article.Id,
            NewsFeedId = article.NewsFeedId,
            Title = article.Title,
            Author = article.Author,
            ArticleGuid = article.ArticleGuid,
            SourceUrl = article.SourceUrl,
            PublishDate = article.PublishDate,
            Description = article.Description,
            Content = article.Content,
            ContentHash = article.ContentHash,
            ImageUrl = article.ImageUrl,
            Categories = article.Categories,
            IsRead = article.IsRead,
            IsStarred = article.IsStarred,
            IsArchived = article.IsArchived,
            ArchivedContent = article.ArchivedContent,
            Added = article.Added
        };
    }
}

public class NewsArticleDetailResource : NewsArticleResource
{
    public new string? Content { get; set; }
    public string? ContentHash { get; set; }
    public string? ArchivedContent { get; set; }
}

public class UnreadCountResult
{
    public int Count { get; set; }
}
