// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Moq;
using Mouseion.Api.Lookup;
using Mouseion.Core.Audiobooks;
using Mouseion.Core.Books;
using Mouseion.Core.Indexers.MyAnonamouse;

namespace Mouseion.Api.Tests.Lookup;

public class TorrentLookupControllerUnitTests
{
    private readonly Mock<IMyAnonamouseIndexer> _indexerMock;
    private readonly TorrentLookupController _controller;

    public TorrentLookupControllerUnitTests()
    {
        _indexerMock = new Mock<IMyAnonamouseIndexer>();
        _controller = new TorrentLookupController(_indexerMock.Object);
    }

    [Fact]
    public async Task SearchBooks_WithNoParameters_ReturnsBadRequest()
    {
        var result = await _controller.SearchBooks(null, null);

        var badRequestResult = Assert.IsType<BadRequestObjectResult>(result.Result);
        Assert.NotNull(badRequestResult.Value);
    }

    [Fact]
    public async Task SearchBooks_WithTitle_ReturnsResults()
    {
        var results = new List<IndexerResult>
        {
            new IndexerResult
            {
                TorrentId = "123",
                Title = "Test Book Torrent",
                Size = 1000000
            }
        };
        _indexerMock.Setup(x => x.SearchBooksAsync(It.IsAny<BookSearchCriteria>(), default))
            .ReturnsAsync(results);

        var result = await _controller.SearchBooks("test", null);

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<TorrentLookupResource>>(okResult.Value);
        Assert.Single(resources);
        Assert.Equal("Test Book Torrent", resources[0].Title);
    }

    [Fact]
    public async Task SearchBooks_WithAuthor_ReturnsResults()
    {
        var results = new List<IndexerResult>
        {
            new IndexerResult
            {
                TorrentId = "456",
                Title = "Author Book Torrent",
                Size = 2000000
            }
        };
        _indexerMock.Setup(x => x.SearchBooksAsync(It.IsAny<BookSearchCriteria>(), default))
            .ReturnsAsync(results);

        var result = await _controller.SearchBooks(null, "author");

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<TorrentLookupResource>>(okResult.Value);
        Assert.Single(resources);
    }

    [Fact]
    public async Task SearchAudiobooks_WithNoParameters_ReturnsBadRequest()
    {
        var result = await _controller.SearchAudiobooks(null, null, null);

        var badRequestResult = Assert.IsType<BadRequestObjectResult>(result.Result);
        Assert.NotNull(badRequestResult.Value);
    }

    [Fact]
    public async Task SearchAudiobooks_WithTitle_ReturnsResults()
    {
        var results = new List<IndexerResult>
        {
            new IndexerResult
            {
                TorrentId = "789",
                Title = "Test Audiobook Torrent",
                Size = 3000000
            }
        };
        _indexerMock.Setup(x => x.SearchAudiobooksAsync(It.IsAny<AudiobookSearchCriteria>(), default))
            .ReturnsAsync(results);

        var result = await _controller.SearchAudiobooks("test", null, null);

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<TorrentLookupResource>>(okResult.Value);
        Assert.Single(resources);
    }

    [Fact]
    public async Task SearchAudiobooks_WithAuthor_ReturnsResults()
    {
        var results = new List<IndexerResult>
        {
            new IndexerResult
            {
                TorrentId = "101",
                Title = "Author Audiobook Torrent",
                Size = 4000000
            }
        };
        _indexerMock.Setup(x => x.SearchAudiobooksAsync(It.IsAny<AudiobookSearchCriteria>(), default))
            .ReturnsAsync(results);

        var result = await _controller.SearchAudiobooks(null, "author", null);

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<TorrentLookupResource>>(okResult.Value);
        Assert.Single(resources);
    }

    [Fact]
    public async Task SearchAudiobooks_WithNarrator_ReturnsResults()
    {
        var results = new List<IndexerResult>
        {
            new IndexerResult
            {
                TorrentId = "102",
                Title = "Narrator Audiobook Torrent",
                Size = 5000000
            }
        };
        _indexerMock.Setup(x => x.SearchAudiobooksAsync(It.IsAny<AudiobookSearchCriteria>(), default))
            .ReturnsAsync(results);

        var result = await _controller.SearchAudiobooks(null, null, "narrator");

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<TorrentLookupResource>>(okResult.Value);
        Assert.Single(resources);
    }
}
