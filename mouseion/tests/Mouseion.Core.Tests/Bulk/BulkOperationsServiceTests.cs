// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Moq;
using Mouseion.Core.Audiobooks;
using Mouseion.Core.Books;
using Mouseion.Core.Bulk;
using Mouseion.Core.Comic;
using Mouseion.Core.Manga;
using Mouseion.Core.Movies;
using Mouseion.Core.News;
using Mouseion.Core.TV;
using Mouseion.Core.Webcomic;

namespace Mouseion.Core.Tests.Bulk;

public class BulkOperationsServiceTests
{
    private readonly Mock<ILogger<BulkOperationsService>> _logger;
    private readonly Mock<IMovieRepository> _movieRepository;
    private readonly Mock<IBookRepository> _bookRepository;
    private readonly Mock<IAudiobookRepository> _audiobookRepository;
    private readonly Mock<ISeriesRepository> _seriesRepository;
    private readonly Mock<IMangaChapterRepository> _mangaChapterRepository;
    private readonly Mock<IWebcomicEpisodeRepository> _webcomicEpisodeRepository;
    private readonly Mock<IComicIssueRepository> _comicIssueRepository;
    private readonly Mock<INewsArticleRepository> _newsArticleRepository;
    private readonly BulkOperationsService _service;

    public BulkOperationsServiceTests()
    {
        _logger = new Mock<ILogger<BulkOperationsService>>();
        _movieRepository = new Mock<IMovieRepository>();
        _bookRepository = new Mock<IBookRepository>();
        _audiobookRepository = new Mock<IAudiobookRepository>();
        _seriesRepository = new Mock<ISeriesRepository>();
        _mangaChapterRepository = new Mock<IMangaChapterRepository>();
        _webcomicEpisodeRepository = new Mock<IWebcomicEpisodeRepository>();
        _comicIssueRepository = new Mock<IComicIssueRepository>();
        _newsArticleRepository = new Mock<INewsArticleRepository>();

        _service = new BulkOperationsService(
            _logger.Object,
            _movieRepository.Object,
            _bookRepository.Object,
            _audiobookRepository.Object,
            _seriesRepository.Object,
            _mangaChapterRepository.Object,
            _webcomicEpisodeRepository.Object,
            _comicIssueRepository.Object,
            _newsArticleRepository.Object);
    }

    [Fact]
    public async Task UpdateMoviesAsync_UpdatesExistingMovies()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie", Monitored = false };
        _movieRepository.Setup(r => r.FindAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(movie);

        var request = new BulkUpdateRequest
        {
            Items = new List<BulkUpdateItem>
            {
                new() { Id = 1, Monitored = true }
            }
        };

        var result = await _service.UpdateMoviesAsync(request);

        Assert.Equal(1, result.Updated);
        Assert.Contains(1, result.UpdatedIds);
        _movieRepository.Verify(r => r.UpdateAsync(It.Is<Movie>(m => m.Monitored), It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task UpdateMoviesAsync_ReportsErrorForMissingMovie()
    {
        _movieRepository.Setup(r => r.FindAsync(999, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Movie?)null);

        var request = new BulkUpdateRequest
        {
            Items = new List<BulkUpdateItem>
            {
                new() { Id = 999, Monitored = true }
            }
        };

        var result = await _service.UpdateMoviesAsync(request);

        Assert.Equal(0, result.Updated);
        Assert.NotNull(result.Errors);
        Assert.Single(result.Errors);
        Assert.Equal(999, result.Errors[0].Id);
    }

    [Fact]
    public async Task DeleteMoviesAsync_DeletesExistingMovies()
    {
        var movie = new Movie { Id = 1, Title = "Test Movie" };
        _movieRepository.Setup(r => r.FindAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(movie);

        var request = new BulkDeleteRequest { Ids = new List<int> { 1 } };

        var result = await _service.DeleteMoviesAsync(request);

        Assert.Equal(1, result.Deleted);
        Assert.Contains(1, result.DeletedIds);
        _movieRepository.Verify(r => r.DeleteAsync(1, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task DeleteMoviesAsync_ReportsErrorForMissingMovie()
    {
        _movieRepository.Setup(r => r.FindAsync(999, It.IsAny<CancellationToken>()))
            .ReturnsAsync((Movie?)null);

        var request = new BulkDeleteRequest { Ids = new List<int> { 999 } };

        var result = await _service.DeleteMoviesAsync(request);

        Assert.Equal(0, result.Deleted);
        Assert.NotNull(result.Errors);
        Assert.Single(result.Errors);
    }

    [Fact]
    public async Task UpdateMoviesAsync_ThrowsForOversizedBatch()
    {
        var items = Enumerable.Range(1, 101)
            .Select(i => new BulkUpdateItem { Id = i })
            .ToList();

        var request = new BulkUpdateRequest { Items = items };

        await Assert.ThrowsAsync<ArgumentException>(() => _service.UpdateMoviesAsync(request));
    }

    [Fact]
    public async Task MarkMangaChaptersReadAsync_MarksAsRead()
    {
        var request = new BulkReadRequest
        {
            Ids = new List<int> { 1, 2, 3 },
            IsRead = true
        };

        var result = await _service.MarkMangaChaptersReadAsync(request);

        Assert.Equal(3, result.Updated);
        _mangaChapterRepository.Verify(r => r.MarkReadAsync(It.IsAny<int>(), It.IsAny<CancellationToken>()), Times.Exactly(3));
    }

    [Fact]
    public async Task MarkMangaChaptersReadAsync_MarksAsUnread()
    {
        var request = new BulkReadRequest
        {
            Ids = new List<int> { 1, 2 },
            IsRead = false
        };

        var result = await _service.MarkMangaChaptersReadAsync(request);

        Assert.Equal(2, result.Updated);
        _mangaChapterRepository.Verify(r => r.MarkUnreadAsync(It.IsAny<int>(), It.IsAny<CancellationToken>()), Times.Exactly(2));
    }

    [Fact]
    public async Task MarkArticlesReadAsync_MarksAsRead()
    {
        var request = new BulkReadRequest
        {
            Ids = new List<int> { 1 },
            IsRead = true
        };

        var result = await _service.MarkArticlesReadAsync(request);

        Assert.Equal(1, result.Updated);
        _newsArticleRepository.Verify(r => r.MarkReadAsync(1, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task UpdateBooksAsync_UpdatesMultipleBooks()
    {
        var book1 = new Book { Id = 1, Title = "Book 1", Monitored = false };
        var book2 = new Book { Id = 2, Title = "Book 2", Monitored = false };

        _bookRepository.Setup(r => r.FindAsync(1, It.IsAny<CancellationToken>())).ReturnsAsync(book1);
        _bookRepository.Setup(r => r.FindAsync(2, It.IsAny<CancellationToken>())).ReturnsAsync(book2);

        var request = new BulkUpdateRequest
        {
            Items = new List<BulkUpdateItem>
            {
                new() { Id = 1, Monitored = true },
                new() { Id = 2, Monitored = true }
            }
        };

        var result = await _service.UpdateBooksAsync(request);

        Assert.Equal(2, result.Updated);
        Assert.Contains(1, result.UpdatedIds);
        Assert.Contains(2, result.UpdatedIds);
    }

    [Fact]
    public async Task UpdateSeriesAsync_UpdatesSeriesProperties()
    {
        var series = new Series { Id = 1, Title = "Test Series", Path = "/old/path" };
        _seriesRepository.Setup(r => r.FindAsync(1, It.IsAny<CancellationToken>()))
            .ReturnsAsync(series);

        var request = new BulkUpdateRequest
        {
            Items = new List<BulkUpdateItem>
            {
                new()
                {
                    Id = 1,
                    Path = "/new/path",
                    RootFolderPath = "/root",
                    Tags = new List<int> { 1, 2 }
                }
            }
        };

        var result = await _service.UpdateSeriesAsync(request);

        Assert.Equal(1, result.Updated);
        Assert.Equal("/new/path", series.Path);
        Assert.Equal("/root", series.RootFolderPath);
        Assert.Contains(1, series.Tags);
        Assert.Contains(2, series.Tags);
    }
}
