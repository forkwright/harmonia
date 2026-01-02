// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Core.Audiobooks;
using Mouseion.Core.Books;
using Mouseion.Core.Indexers.MyAnonamouse;
using Mouseion.Core.MetadataSource;

namespace Mouseion.Api.Lookup;

[ApiController]
[Route("api/v3/lookup")]
[Authorize]
public class LookupController : ControllerBase
{
    private readonly IProvideBookInfo _bookInfoProvider;
    private readonly IProvideAudiobookInfo _audiobookInfoProvider;
    private readonly MyAnonamouseIndexer _myAnonamouseIndexer;

    public LookupController(
        IProvideBookInfo bookInfoProvider,
        IProvideAudiobookInfo audiobookInfoProvider,
        MyAnonamouseIndexer myAnonamouseIndexer)
    {
        _bookInfoProvider = bookInfoProvider;
        _audiobookInfoProvider = audiobookInfoProvider;
        _myAnonamouseIndexer = myAnonamouseIndexer;
    }

    [HttpGet("books")]
    public async Task<ActionResult<List<BookLookupResource>>> SearchBooks([FromQuery] string? title, [FromQuery] string? author, [FromQuery] string? isbn, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(title) && string.IsNullOrWhiteSpace(author) && string.IsNullOrWhiteSpace(isbn))
        {
            return BadRequest(new { error = "At least one search parameter (title, author, or isbn) is required" });
        }

        var results = new List<Book>();

        if (!string.IsNullOrWhiteSpace(title))
        {
            results.AddRange(await _bookInfoProvider.SearchByTitleAsync(title, ct).ConfigureAwait(false));
        }

        if (!string.IsNullOrWhiteSpace(author))
        {
            results.AddRange(await _bookInfoProvider.SearchByAuthorAsync(author, ct).ConfigureAwait(false));
        }

        if (!string.IsNullOrWhiteSpace(isbn))
        {
            results.AddRange(await _bookInfoProvider.SearchByIsbnAsync(isbn, ct).ConfigureAwait(false));
        }

        return Ok(results.Distinct().Select(ToBookLookup).ToList());
    }

    [HttpGet("books/{id}")]
    public async Task<ActionResult<BookLookupResource>> GetBookById(string id, CancellationToken ct = default)
    {
        var book = await _bookInfoProvider.GetByExternalIdAsync(id, ct).ConfigureAwait(false);
        if (book == null)
        {
            return NotFound(new { error = $"Book with ID {id} not found" });
        }

        return Ok(ToBookLookup(book));
    }

    [HttpGet("books/trending")]
    public async Task<ActionResult<List<BookLookupResource>>> GetTrendingBooks(CancellationToken ct = default)
    {
        var books = await _bookInfoProvider.GetTrendingAsync(ct).ConfigureAwait(false);
        return Ok(books.Select(ToBookLookup).ToList());
    }

    [HttpGet("audiobooks")]
    public async Task<ActionResult<List<AudiobookLookupResource>>> SearchAudiobooks(
        [FromQuery] string? title,
        [FromQuery] string? author,
        [FromQuery] string? narrator,
        [FromQuery] string? asin,
        CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(title) && string.IsNullOrWhiteSpace(author) &&
            string.IsNullOrWhiteSpace(narrator) && string.IsNullOrWhiteSpace(asin))
        {
            return BadRequest(new { error = "At least one search parameter (title, author, narrator, or asin) is required" });
        }

        var results = new List<Audiobook>();

        if (!string.IsNullOrWhiteSpace(title))
        {
            results.AddRange(await _audiobookInfoProvider.SearchByTitleAsync(title, ct).ConfigureAwait(false));
        }

        if (!string.IsNullOrWhiteSpace(author))
        {
            results.AddRange(await _audiobookInfoProvider.SearchByAuthorAsync(author, ct).ConfigureAwait(false));
        }

        if (!string.IsNullOrWhiteSpace(narrator))
        {
            results.AddRange(await _audiobookInfoProvider.SearchByNarratorAsync(narrator, ct).ConfigureAwait(false));
        }

        return Ok(results.Distinct().Select(ToAudiobookLookup).ToList());
    }

    [HttpGet("audiobooks/{asin}")]
    public async Task<ActionResult<AudiobookLookupResource>> GetAudiobookByAsin(string asin, CancellationToken ct = default)
    {
        var audiobook = await _audiobookInfoProvider.GetByAsinAsync(asin, ct).ConfigureAwait(false);
        if (audiobook == null)
        {
            return NotFound(new { error = $"Audiobook with ASIN {asin} not found" });
        }

        return Ok(ToAudiobookLookup(audiobook));
    }

    [HttpGet("audiobooks/narrator/{narrator}")]
    public async Task<ActionResult<List<AudiobookLookupResource>>> SearchByNarrator(string narrator, CancellationToken ct = default)
    {
        var audiobooks = await _audiobookInfoProvider.SearchByNarratorAsync(narrator, ct).ConfigureAwait(false);
        return Ok(audiobooks.Select(ToAudiobookLookup).ToList());
    }

    [HttpGet("torrents/books")]
    public async Task<ActionResult<List<TorrentLookupResource>>> SearchBookTorrents([FromQuery] string? title, [FromQuery] string? author, CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(title) && string.IsNullOrWhiteSpace(author))
        {
            return BadRequest(new { error = "At least one search parameter (title or author) is required" });
        }

        var criteria = new BookSearchCriteria
        {
            Title = title,
            Author = author
        };

        var results = await _myAnonamouseIndexer.SearchBooksAsync(criteria, ct).ConfigureAwait(false);
        return Ok(results.Select(ToTorrentLookup).ToList());
    }

    [HttpGet("torrents/audiobooks")]
    public async Task<ActionResult<List<TorrentLookupResource>>> SearchAudiobookTorrents(
        [FromQuery] string? title,
        [FromQuery] string? author,
        [FromQuery] string? narrator,
        CancellationToken ct = default)
    {
        if (string.IsNullOrWhiteSpace(title) && string.IsNullOrWhiteSpace(author) && string.IsNullOrWhiteSpace(narrator))
        {
            return BadRequest(new { error = "At least one search parameter (title, author, or narrator) is required" });
        }

        var criteria = new AudiobookSearchCriteria
        {
            Title = title,
            Author = author,
            Narrator = narrator
        };

        var results = await _myAnonamouseIndexer.SearchAudiobooksAsync(criteria, ct).ConfigureAwait(false);
        return Ok(results.Select(ToTorrentLookup).ToList());
    }

    private static BookLookupResource ToBookLookup(Book book)
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

    private static AudiobookLookupResource ToAudiobookLookup(Audiobook audiobook)
    {
        return new AudiobookLookupResource
        {
            Title = audiobook.Title,
            Year = audiobook.Year,
            ForeignAudiobookId = audiobook.Metadata.ForeignAudiobookId,
            AudnexusId = audiobook.Metadata.AudnexusId,
            AudibleId = audiobook.Metadata.AudibleId,
            Asin = audiobook.Metadata.Asin,
            Isbn = audiobook.Metadata.Isbn,
            Isbn13 = audiobook.Metadata.Isbn13,
            Description = audiobook.Metadata.Description,
            Narrator = audiobook.Metadata.Narrator,
            Narrators = audiobook.Metadata.Narrators,
            DurationMinutes = audiobook.Metadata.DurationMinutes,
            IsAbridged = audiobook.Metadata.IsAbridged,
            Publisher = audiobook.Metadata.Publisher,
            Language = audiobook.Metadata.Language,
            Genres = audiobook.Metadata.Genres,
            ReleaseDate = audiobook.Metadata.ReleaseDate
        };
    }

    private static TorrentLookupResource ToTorrentLookup(IndexerResult result)
    {
        return new TorrentLookupResource
        {
            TorrentId = result.TorrentId,
            Title = result.Title,
            Author = result.Author,
            Category = result.Category,
            Size = result.Size,
            Seeders = result.Seeders,
            Leechers = result.Leechers,
            PublishDate = result.PublishDate,
            DownloadUrl = result.DownloadUrl,
            InfoUrl = result.InfoUrl,
            IsFreeleech = result.IsFreeleech
        };
    }
}

public class BookLookupResource
{
    public string Title { get; set; } = null!;
    public int Year { get; set; }
    public string? ForeignBookId { get; set; }
    public string? OpenLibraryId { get; set; }
    public string? GoodreadsId { get; set; }
    public string? GoogleBooksId { get; set; }
    public string? Isbn { get; set; }
    public string? Isbn13 { get; set; }
    public string? Description { get; set; }
    public int? PageCount { get; set; }
    public string? Publisher { get; set; }
    public string? Language { get; set; }
    public List<string> Genres { get; set; } = new();
    public DateTime? ReleaseDate { get; set; }
}

public class AudiobookLookupResource
{
    public string Title { get; set; } = null!;
    public int Year { get; set; }
    public string? ForeignAudiobookId { get; set; }
    public string? AudnexusId { get; set; }
    public string? AudibleId { get; set; }
    public string? Asin { get; set; }
    public string? Isbn { get; set; }
    public string? Isbn13 { get; set; }
    public string? Description { get; set; }
    public string? Narrator { get; set; }
    public List<string> Narrators { get; set; } = new();
    public int? DurationMinutes { get; set; }
    public bool IsAbridged { get; set; }
    public string? Publisher { get; set; }
    public string? Language { get; set; }
    public List<string> Genres { get; set; } = new();
    public DateTime? ReleaseDate { get; set; }
}

public class TorrentLookupResource
{
    public string TorrentId { get; set; } = null!;
    public string Title { get; set; } = null!;
    public string? Author { get; set; }
    public string? Category { get; set; }
    public long Size { get; set; }
    public int Seeders { get; set; }
    public int Leechers { get; set; }
    public DateTime? PublishDate { get; set; }
    public string? DownloadUrl { get; set; }
    public string? InfoUrl { get; set; }
    public bool IsFreeleech { get; set; }
}
