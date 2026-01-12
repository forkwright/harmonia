// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Books;
using Mouseion.Core.MetadataSource;

namespace Mouseion.Api.Lookup;

[ApiController]
[Route("api/v3/lookup/books")]
[Authorize]
public class BookLookupController : ControllerBase
{
    private readonly IProvideBookInfo _bookInfoProvider;

    public BookLookupController(IProvideBookInfo bookInfoProvider)
    {
        _bookInfoProvider = bookInfoProvider;
    }

    [HttpGet]
    public async Task<ActionResult<List<BookLookupResource>>> Search(
        [FromQuery] string? title,
        [FromQuery] string? author,
        [FromQuery] string? isbn,
        CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(title) && string.IsNullOrWhiteSpace(author) && string.IsNullOrWhiteSpace(isbn))
        {
            return BadRequest(new { error = "At least one search parameter (title, author, or isbn) is required" });
        }

        var tasks = new List<Task<List<Book>>>();

        if (!string.IsNullOrWhiteSpace(title))
            tasks.Add(_bookInfoProvider.SearchByTitleAsync(title, ct));

        if (!string.IsNullOrWhiteSpace(author))
            tasks.Add(_bookInfoProvider.SearchByAuthorAsync(author, ct));

        if (!string.IsNullOrWhiteSpace(isbn))
            tasks.Add(_bookInfoProvider.SearchByIsbnAsync(isbn, ct));

        var searchResults = await Task.WhenAll(tasks).ConfigureAwait(false);
        var results = searchResults.SelectMany(x => x).Distinct();

        return Ok(results.Select(ToResource).ToList());
    }

    [HttpGet("{id}")]
    public async Task<ActionResult<BookLookupResource>> GetById(string id, CancellationToken ct = default)
    {
        var book = await _bookInfoProvider.GetByExternalIdAsync(id, ct).ConfigureAwait(false);
        if (book == null)
        {
            return NotFound(new { error = $"Book with ID {id} not found" });
        }

        return Ok(ToResource(book));
    }

    [HttpGet("trending")]
    public async Task<ActionResult<List<BookLookupResource>>> GetTrending(CancellationToken ct = default)
    {
        var books = await _bookInfoProvider.GetTrendingAsync(ct).ConfigureAwait(false);
        return Ok(books.Select(ToResource).ToList());
    }

    private static BookLookupResource ToResource(Book book)
    {
        return new BookLookupResource
        {
            Title = book.Title,
            Year = book.Year,
            ForeignBookId = book.Metadata.ForeignBookId,
            OpenLibraryId = book.Metadata.OpenLibraryId,
            GoodreadsId = book.Metadata.GoodreadsId,
            GoogleBooksId = book.Metadata.GoogleBooksId,
            Isbn = book.Metadata.Isbn,
            Isbn13 = book.Metadata.Isbn13,
            Description = book.Metadata.Description,
            PageCount = book.Metadata.PageCount,
            Publisher = book.Metadata.Publisher,
            Language = book.Metadata.Language,
            Genres = book.Metadata.Genres,
            ReleaseDate = book.Metadata.ReleaseDate
        };
    }
}
