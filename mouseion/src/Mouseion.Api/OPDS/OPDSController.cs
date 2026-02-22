// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.MediaItems;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.OPDS;

namespace Mouseion.Api.OPDS;

/// <summary>
/// OPDS 1.2 catalog endpoints for external reading applications.
/// Serves Atom feeds compatible with KOReader, Calibre, Moon+ Reader, etc.
///
/// Catalog root: GET /opds/v1.2/catalog
/// Search descriptor: GET /opds/v1.2/search.xml
/// </summary>
[ApiController]
[Route("opds/v1.2")]
[Produces("application/atom+xml;profile=opds-catalog")]
public class OPDSController : ControllerBase
{
    private readonly IOPDSFeedBuilder _feedBuilder;
    private readonly IMediaItemRepository _mediaItemRepo;

    private const int DefaultPageSize = 25;
    private const int MaxPageSize = 100;

    public OPDSController(IOPDSFeedBuilder feedBuilder, IMediaItemRepository mediaItemRepo)
    {
        _feedBuilder = feedBuilder;
        _mediaItemRepo = mediaItemRepo;
    }

    /// <summary>
    /// OPDS catalog root — navigation feed with links to Books, Comics, Manga, Recent.
    /// </summary>
    [HttpGet("catalog")]
    public IActionResult GetCatalog()
    {
        var baseUrl = GetBaseUrl();
        var feed = _feedBuilder.BuildCatalogRoot(baseUrl);
        return Content(feed.ToString(), "application/atom+xml;profile=opds-catalog;kind=navigation");
    }

    /// <summary>
    /// Books navigation feed.
    /// </summary>
    [HttpGet("books")]
    public IActionResult GetBooks()
    {
        var baseUrl = GetBaseUrl();
        var feed = _feedBuilder.BuildNavigationFeed(baseUrl, "books", "Books");
        return Content(feed.ToString(), "application/atom+xml;profile=opds-catalog;kind=navigation");
    }

    /// <summary>
    /// All books — acquisition feed with pagination.
    /// </summary>
    [HttpGet("books/all")]
    public async Task<IActionResult> GetAllBooks([FromQuery] int page = 1, [FromQuery] int pageSize = DefaultPageSize, CancellationToken ct = default)
    {
        return await BuildAcquisitionFeed(MediaType.Book, "Books", page, pageSize, ct);
    }

    /// <summary>
    /// Recently added books.
    /// </summary>
    [HttpGet("books/recent")]
    public async Task<IActionResult> GetRecentBooks([FromQuery] int page = 1, [FromQuery] int pageSize = DefaultPageSize, CancellationToken ct = default)
    {
        return await BuildAcquisitionFeed(MediaType.Book, "Recent Books", page, pageSize, ct);
    }

    /// <summary>
    /// Comics navigation feed.
    /// </summary>
    [HttpGet("comics")]
    public IActionResult GetComics()
    {
        var baseUrl = GetBaseUrl();
        var feed = _feedBuilder.BuildNavigationFeed(baseUrl, "comics", "Comics");
        return Content(feed.ToString(), "application/atom+xml;profile=opds-catalog;kind=navigation");
    }

    /// <summary>
    /// All comics — acquisition feed with pagination.
    /// </summary>
    [HttpGet("comics/all")]
    public async Task<IActionResult> GetAllComics([FromQuery] int page = 1, [FromQuery] int pageSize = DefaultPageSize, CancellationToken ct = default)
    {
        return await BuildAcquisitionFeed(MediaType.Comic, "Comics", page, pageSize, ct);
    }

    /// <summary>
    /// Recently added comics.
    /// </summary>
    [HttpGet("comics/recent")]
    public async Task<IActionResult> GetRecentComics([FromQuery] int page = 1, [FromQuery] int pageSize = DefaultPageSize, CancellationToken ct = default)
    {
        return await BuildAcquisitionFeed(MediaType.Comic, "Recent Comics", page, pageSize, ct);
    }

    /// <summary>
    /// Manga navigation feed.
    /// </summary>
    [HttpGet("manga")]
    public IActionResult GetManga()
    {
        var baseUrl = GetBaseUrl();
        var feed = _feedBuilder.BuildNavigationFeed(baseUrl, "manga", "Manga");
        return Content(feed.ToString(), "application/atom+xml;profile=opds-catalog;kind=navigation");
    }

    /// <summary>
    /// All manga — acquisition feed with pagination.
    /// </summary>
    [HttpGet("manga/all")]
    public async Task<IActionResult> GetAllManga([FromQuery] int page = 1, [FromQuery] int pageSize = DefaultPageSize, CancellationToken ct = default)
    {
        return await BuildAcquisitionFeed(MediaType.Manga, "Manga", page, pageSize, ct);
    }

    /// <summary>
    /// Recently added manga.
    /// </summary>
    [HttpGet("manga/recent")]
    public async Task<IActionResult> GetRecentManga([FromQuery] int page = 1, [FromQuery] int pageSize = DefaultPageSize, CancellationToken ct = default)
    {
        return await BuildAcquisitionFeed(MediaType.Manga, "Recent Manga", page, pageSize, ct);
    }

    /// <summary>
    /// Recently added items across all readable media types.
    /// </summary>
    [HttpGet("recent")]
    public async Task<IActionResult> GetRecent([FromQuery] int page = 1, [FromQuery] int pageSize = DefaultPageSize, CancellationToken ct = default)
    {
        // Get recent across Book, Comic, Manga types
        pageSize = Math.Min(pageSize, MaxPageSize);
        var baseUrl = GetBaseUrl();

        var bookCount = await _mediaItemRepo.CountAsync(MediaType.Book, ct);
        var comicCount = await _mediaItemRepo.CountAsync(MediaType.Comic, ct);
        var mangaCount = await _mediaItemRepo.CountAsync(MediaType.Manga, ct);
        var totalItems = bookCount + comicCount + mangaCount;

        // Get items from all readable types, merged by Added date
        var books = await _mediaItemRepo.GetPageAsync(page, pageSize, MediaType.Book, ct);
        var comics = await _mediaItemRepo.GetPageAsync(page, pageSize, MediaType.Comic, ct);
        var manga = await _mediaItemRepo.GetPageAsync(page, pageSize, MediaType.Manga, ct);

        var items = books.Concat(comics).Concat(manga)
            .OrderByDescending(i => i.Added)
            .Take(pageSize)
            .ToList();

        var feed = _feedBuilder.BuildAcquisitionFeed(baseUrl, items, "Recently Added", page, pageSize, totalItems);
        return Content(feed.ToString(), "application/atom+xml;profile=opds-catalog;kind=acquisition");
    }

    /// <summary>
    /// Search across all readable media types.
    /// </summary>
    [HttpGet("search")]
    public async Task<IActionResult> Search([FromQuery] string q = "", [FromQuery] int page = 1, [FromQuery] int pageSize = DefaultPageSize, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(q))
            return BadRequest("Search query required");

        pageSize = Math.Min(pageSize, MaxPageSize);
        var baseUrl = GetBaseUrl();

        // Search across readable types
        var allItems = new List<MediaItemSummary>();
        foreach (var mediaType in new[] { MediaType.Book, MediaType.Comic, MediaType.Manga })
        {
            var items = await _mediaItemRepo.GetPageAsync(1, 500, mediaType, ct);
            allItems.AddRange(items.Where(i =>
                i.Title.Contains(q, StringComparison.OrdinalIgnoreCase)));
        }

        var totalItems = allItems.Count;
        var pageItems = allItems
            .OrderByDescending(i => i.Added)
            .Skip((page - 1) * pageSize)
            .Take(pageSize)
            .ToList();

        var feed = _feedBuilder.BuildAcquisitionFeed(baseUrl, pageItems, $"Search: {q}", page, pageSize, totalItems);
        return Content(feed.ToString(), "application/atom+xml;profile=opds-catalog;kind=acquisition");
    }

    /// <summary>
    /// OpenSearch descriptor for OPDS search.
    /// </summary>
    [HttpGet("search.xml")]
    [Produces("application/opensearchdescription+xml")]
    public IActionResult GetSearchDescriptor()
    {
        var baseUrl = GetBaseUrl();
        var descriptor = _feedBuilder.BuildSearchDescriptor(baseUrl);
        return Content(descriptor.ToString(), "application/opensearchdescription+xml");
    }

    private async Task<IActionResult> BuildAcquisitionFeed(MediaType mediaType, string title, int page, int pageSize, CancellationToken ct)
    {
        pageSize = Math.Min(pageSize, MaxPageSize);
        var baseUrl = GetBaseUrl();

        var totalItems = await _mediaItemRepo.CountAsync(mediaType, ct);
        var items = await _mediaItemRepo.GetPageAsync(page, pageSize, mediaType, ct);

        var feed = _feedBuilder.BuildAcquisitionFeed(baseUrl, items, title, page, pageSize, totalItems);
        return Content(feed.ToString(), "application/atom+xml;profile=opds-catalog;kind=acquisition");
    }

    private string GetBaseUrl()
    {
        return $"{Request.Scheme}://{Request.Host}";
    }
}
