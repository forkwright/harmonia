// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;
using Moq;
using Mouseion.Api.Lookup;
using Mouseion.Core.Books;
using Mouseion.Core.MetadataSource;

namespace Mouseion.Api.Tests.Lookup;

public class BookLookupControllerUnitTests
{
    private readonly Mock<IProvideBookInfo> _bookInfoProviderMock;
    private readonly BookLookupController _controller;

    public BookLookupControllerUnitTests()
    {
        _bookInfoProviderMock = new Mock<IProvideBookInfo>();
        _controller = new BookLookupController(_bookInfoProviderMock.Object);
    }

    [Fact]
    public async Task Search_WithNoParameters_ReturnsBadRequest()
    {
        var result = await _controller.Search(null, null, null);

        var badRequestResult = Assert.IsType<BadRequestObjectResult>(result.Result);
        Assert.NotNull(badRequestResult.Value);
    }

    [Fact]
    public async Task Search_WithTitle_ReturnsResults()
    {
        var books = new List<Book>
        {
            new Book { Title = "Test Book", Year = 2025, Metadata = new BookMetadata() }
        };
        _bookInfoProviderMock.Setup(x => x.SearchByTitleAsync("test", default))
            .ReturnsAsync(books);

        var result = await _controller.Search("test", null, null);

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<BookLookupResource>>(okResult.Value);
        Assert.Single(resources);
        Assert.Equal("Test Book", resources[0].Title);
    }

    [Fact]
    public async Task Search_WithAuthor_ReturnsResults()
    {
        var books = new List<Book>
        {
            new Book { Title = "Author Book", Year = 2025, Metadata = new BookMetadata() }
        };
        _bookInfoProviderMock.Setup(x => x.SearchByAuthorAsync("author", default))
            .ReturnsAsync(books);

        var result = await _controller.Search(null, "author", null);

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<BookLookupResource>>(okResult.Value);
        Assert.Single(resources);
    }

    [Fact]
    public async Task Search_WithIsbn_ReturnsResults()
    {
        var books = new List<Book>
        {
            new Book { Title = "ISBN Book", Year = 2025, Metadata = new BookMetadata() }
        };
        _bookInfoProviderMock.Setup(x => x.SearchByIsbnAsync("1234567890", default))
            .ReturnsAsync(books);

        var result = await _controller.Search(null, null, "1234567890");

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<BookLookupResource>>(okResult.Value);
        Assert.Single(resources);
    }

    [Fact]
    public async Task GetById_WhenBookExists_ReturnsBook()
    {
        var book = new Book { Title = "Found Book", Year = 2025, Metadata = new BookMetadata() };
        _bookInfoProviderMock.Setup(x => x.GetByExternalIdAsync("123", default))
            .ReturnsAsync(book);

        var result = await _controller.GetById("123");

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resource = Assert.IsType<BookLookupResource>(okResult.Value);
        Assert.Equal("Found Book", resource.Title);
    }

    [Fact]
    public async Task GetById_WhenBookNotFound_ReturnsNotFound()
    {
        _bookInfoProviderMock.Setup(x => x.GetByExternalIdAsync("999", default))
            .ReturnsAsync(null as Book);

        var result = await _controller.GetById("999");

        Assert.IsType<NotFoundObjectResult>(result.Result);
    }

    [Fact]
    public async Task GetTrending_ReturnsBooks()
    {
        var books = new List<Book>
        {
            new Book { Title = "Trending Book", Year = 2025, Metadata = new BookMetadata() }
        };
        _bookInfoProviderMock.Setup(x => x.GetTrendingAsync(default))
            .ReturnsAsync(books);

        var result = await _controller.GetTrending();

        var okResult = Assert.IsType<OkObjectResult>(result.Result);
        var resources = Assert.IsType<List<BookLookupResource>>(okResult.Value);
        Assert.Single(resources);
    }
}
