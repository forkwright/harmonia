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
using Mouseion.Core.Books;

namespace Mouseion.Api.Books;

[ApiController]
[Route("api/v3/books")]
[Authorize]
public class BookController : ControllerBase
{
    private readonly IBookRepository _bookRepository;
    private readonly IAddBookService _addBookService;
    private readonly IBookStatisticsService _statisticsService;

    public BookController(
        IBookRepository bookRepository,
        IAddBookService addBookService,
        IBookStatisticsService statisticsService)
    {
        _bookRepository = bookRepository;
        _addBookService = addBookService;
        _statisticsService = statisticsService;
    }

    [HttpGet]
    public async Task<ActionResult<PagedResult<BookResource>>> GetBooks(
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50,
        CancellationToken ct = default)
    {
        if (page < 1) page = 1;
        if (pageSize < 1) pageSize = 50;
        if (pageSize > 250) pageSize = 250;

        var totalCount = await _bookRepository.CountAsync(ct).ConfigureAwait(false);
        var books = await _bookRepository.GetPageAsync(page, pageSize, ct).ConfigureAwait(false);

        return Ok(new PagedResult<BookResource>
        {
            Items = books.Select(ToResource),
            Page = page,
            PageSize = pageSize,
            TotalCount = totalCount
        });
    }

    [HttpGet("{id:int}")]
    public async Task<ActionResult<BookResource>> GetBook(int id, CancellationToken ct = default)
    {
        var book = await _bookRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (book == null)
        {
            return NotFound(new { error = $"Book {id} not found" });
        }

        return Ok(ToResource(book));
    }

    [HttpGet("author/{authorId:int}")]
    public async Task<ActionResult<List<BookResource>>> GetBooksByAuthor(int authorId, CancellationToken ct = default)
    {
        var books = await _bookRepository.GetByAuthorIdAsync(authorId, ct).ConfigureAwait(false);
        return Ok(books.Select(ToResource).ToList());
    }

    [HttpGet("series/{seriesId:int}")]
    public async Task<ActionResult<List<BookResource>>> GetBooksBySeries(int seriesId, CancellationToken ct = default)
    {
        var books = await _bookRepository.GetBySeriesIdAsync(seriesId, ct).ConfigureAwait(false);
        return Ok(books.Select(ToResource).ToList());
    }

    [HttpGet("statistics")]
    public async Task<ActionResult<BookStatistics>> GetStatistics(CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetStatisticsAsync(ct).ConfigureAwait(false);
        return Ok(stats);
    }

    [HttpGet("statistics/author/{authorId:int}")]
    public async Task<ActionResult<BookStatistics>> GetAuthorStatistics(int authorId, CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetAuthorStatisticsAsync(authorId, ct).ConfigureAwait(false);
        return Ok(stats);
    }

    [HttpGet("statistics/series/{seriesId:int}")]
    public async Task<ActionResult<BookStatistics>> GetSeriesStatistics(int seriesId, CancellationToken ct = default)
    {
        var stats = await _statisticsService.GetSeriesStatisticsAsync(seriesId, ct).ConfigureAwait(false);
        return Ok(stats);
    }

    [HttpPost]
    public async Task<ActionResult<BookResource>> AddBook([FromBody] BookResource resource, CancellationToken ct = default)
    {
        var book = ToModel(resource);
        var added = await _addBookService.AddBookAsync(book, ct).ConfigureAwait(false);
        return CreatedAtAction(nameof(GetBook), new { id = added.Id }, ToResource(added));
    }

    [HttpPost("batch")]
    public async Task<ActionResult<List<BookResource>>> AddBooks([FromBody] List<BookResource> resources, CancellationToken ct = default)
    {
        var books = resources.Select(ToModel).ToList();
        var added = await _addBookService.AddBooksAsync(books, ct).ConfigureAwait(false);
        return Ok(added.Select(ToResource).ToList());
    }

    [HttpPut("{id:int}")]
    public async Task<ActionResult<BookResource>> UpdateBook(int id, [FromBody] BookResource resource, CancellationToken ct = default)
    {
        var book = await _bookRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (book == null)
        {
            return NotFound(new { error = $"Book {id} not found" });
        }

        book.Title = resource.Title;
        book.Year = resource.Year;
        book.Monitored = resource.Monitored;
        book.QualityProfileId = resource.QualityProfileId;
        book.AuthorId = resource.AuthorId;
        book.BookSeriesId = resource.BookSeriesId;
        book.Tags = resource.Tags?.ToHashSet() ?? new HashSet<int>();
        book.Metadata = ToMetadata(resource.Metadata);

        var updated = await _bookRepository.UpdateAsync(book, ct).ConfigureAwait(false);
        return Ok(ToResource(updated));
    }

    [HttpDelete("{id:int}")]
    public async Task<IActionResult> DeleteBook(int id, CancellationToken ct = default)
    {
        var book = await _bookRepository.FindAsync(id, ct).ConfigureAwait(false);
        if (book == null)
        {
            return NotFound(new { error = $"Book {id} not found" });
        }

        await _bookRepository.DeleteAsync(id, ct).ConfigureAwait(false);
        return NoContent();
    }

    private static BookResource ToResource(Book book)
    {
        return new BookResource
        {
            Id = book.Id,
            Title = book.Title,
            Year = book.Year,
            Monitored = book.Monitored,
            QualityProfileId = book.QualityProfileId,
            Added = book.Added,
            AuthorId = book.AuthorId,
            BookSeriesId = book.BookSeriesId,
            Tags = book.Tags?.ToList(),
            Metadata = new BookMetadataResource
            {
                Description = book.Metadata.Description,
                ForeignBookId = book.Metadata.ForeignBookId,
                GoodreadsId = book.Metadata.GoodreadsId,
                OpenLibraryId = book.Metadata.OpenLibraryId,
                GoogleBooksId = book.Metadata.GoogleBooksId,
                Isbn = book.Metadata.Isbn,
                Isbn13 = book.Metadata.Isbn13,
                Asin = book.Metadata.Asin,
                PageCount = book.Metadata.PageCount,
                ReleaseDate = book.Metadata.ReleaseDate,
                Publisher = book.Metadata.Publisher,
                Language = book.Metadata.Language,
                Genres = book.Metadata.Genres,
                SeriesPosition = book.Metadata.SeriesPosition
            }
        };
    }

    private static Book ToModel(BookResource resource)
    {
        return new Book
        {
            Id = resource.Id,
            Title = resource.Title,
            Year = resource.Year,
            Monitored = resource.Monitored,
            QualityProfileId = resource.QualityProfileId,
            Added = resource.Added,
            AuthorId = resource.AuthorId,
            BookSeriesId = resource.BookSeriesId,
            Tags = resource.Tags?.ToHashSet() ?? new HashSet<int>(),
            Metadata = ToMetadata(resource.Metadata)
        };
    }

    private static BookMetadata ToMetadata(BookMetadataResource? resource)
    {
        if (resource == null)
        {
            return new BookMetadata();
        }

        return new BookMetadata
        {
            Description = resource.Description,
            ForeignBookId = resource.ForeignBookId,
            GoodreadsId = resource.GoodreadsId,
            OpenLibraryId = resource.OpenLibraryId,
            GoogleBooksId = resource.GoogleBooksId,
            Isbn = resource.Isbn,
            Isbn13 = resource.Isbn13,
            Asin = resource.Asin,
            PageCount = resource.PageCount,
            ReleaseDate = resource.ReleaseDate,
            Publisher = resource.Publisher,
            Language = resource.Language,
            Genres = resource.Genres ?? new List<string>(),
            SeriesPosition = resource.SeriesPosition
        };
    }
}

public class BookResource
{
    public int Id { get; set; }
    public string Title { get; set; } = null!;
    public int Year { get; set; }
    public bool Monitored { get; set; }
    public int QualityProfileId { get; set; }
    public DateTime Added { get; set; }
    public int? AuthorId { get; set; }
    public int? BookSeriesId { get; set; }
    public List<int>? Tags { get; set; }
    public BookMetadataResource Metadata { get; set; } = new();
}

public class BookMetadataResource
{
    public string? Description { get; set; }
    public string? ForeignBookId { get; set; }
    public string? GoodreadsId { get; set; }
    public string? OpenLibraryId { get; set; }
    public string? GoogleBooksId { get; set; }
    public string? Isbn { get; set; }
    public string? Isbn13 { get; set; }
    public string? Asin { get; set; }
    public int? PageCount { get; set; }
    public DateTime? ReleaseDate { get; set; }
    public string? Publisher { get; set; }
    public string? Language { get; set; }
    public List<string> Genres { get; set; } = new();
    public int? SeriesPosition { get; set; }
}
