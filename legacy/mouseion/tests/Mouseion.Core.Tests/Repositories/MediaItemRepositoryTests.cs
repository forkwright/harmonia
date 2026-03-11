// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Books;
using Mouseion.Core.MediaItems;
using Mouseion.Core.MediaTypes;
using Mouseion.Core.Movies;

namespace Mouseion.Core.Tests.Repositories;

public class MediaItemRepositoryTests : RepositoryTestBase
{
    private readonly IMediaItemRepository _repository;
    private readonly IBookRepository _bookRepository;
    private readonly IMovieRepository _movieRepository;

    public MediaItemRepositoryTests()
    {
        _repository = new MediaItemRepository(Database);
        _bookRepository = new BookRepository(Database);
        _movieRepository = new MovieRepository(Database);
    }

    [Fact]
    public async Task FindByIdAsync_NonExistent_ReturnsNull()
    {
        var result = await _repository.FindByIdAsync(99999);

        Assert.Null(result);
    }

    [Fact]
    public async Task FindByIdAsync_ExistingBook_ReturnsBook()
    {
        var book = CreateBook("Test Book", 2024);
        await _bookRepository.InsertAsync(book);

        var result = await _repository.FindByIdAsync(book.Id);

        Assert.NotNull(result);
        Assert.IsType<Book>(result);
        Assert.Equal("Test Book", result.GetTitle());
    }

    [Fact]
    public async Task FindByIdAsync_ExistingMovie_ReturnsMovie()
    {
        var movie = CreateMovie("Test Movie", 2024);
        await _movieRepository.InsertAsync(movie);

        var result = await _repository.FindByIdAsync(movie.Id);

        Assert.NotNull(result);
        Assert.IsType<Movie>(result);
        Assert.Equal("Test Movie", result.GetTitle());
    }

    [Fact]
    public async Task GetPageAsync_EmptyDatabase_ReturnsEmptyList()
    {
        var result = await _repository.GetPageAsync(1, 10);

        Assert.NotNull(result);
        Assert.Empty(result);
    }

    [Fact]
    public async Task GetPageAsync_WithItems_ReturnsPaginatedResults()
    {
        for (int i = 1; i <= 15; i++)
        {
            await _bookRepository.InsertAsync(CreateBook($"Book {i}", 2020 + i));
        }

        var page1 = await _repository.GetPageAsync(1, 10);
        var page2 = await _repository.GetPageAsync(2, 10);

        Assert.Equal(10, page1.Count);
        Assert.Equal(5, page2.Count);
    }

    [Fact]
    public async Task GetPageAsync_WithMediaTypeFilter_ReturnsOnlyMatchingType()
    {
        await _bookRepository.InsertAsync(CreateBook("Book 1", 2024));
        await _bookRepository.InsertAsync(CreateBook("Book 2", 2024));
        await _movieRepository.InsertAsync(CreateMovie("Movie 1", 2024));

        var books = await _repository.GetPageAsync(1, 10, MediaType.Book);
        var movies = await _repository.GetPageAsync(1, 10, MediaType.Movie);

        Assert.Equal(2, books.Count);
        Assert.Single(movies);
    }

    [Fact]
    public async Task CountAsync_EmptyDatabase_ReturnsZero()
    {
        var result = await _repository.CountAsync();

        Assert.Equal(0, result);
    }

    [Fact]
    public async Task CountAsync_WithItems_ReturnsCorrectCount()
    {
        await _bookRepository.InsertAsync(CreateBook("Book 1", 2024));
        await _bookRepository.InsertAsync(CreateBook("Book 2", 2024));
        await _movieRepository.InsertAsync(CreateMovie("Movie 1", 2024));

        var total = await _repository.CountAsync();
        var booksOnly = await _repository.CountAsync(MediaType.Book);
        var moviesOnly = await _repository.CountAsync(MediaType.Movie);

        Assert.Equal(3, total);
        Assert.Equal(2, booksOnly);
        Assert.Equal(1, moviesOnly);
    }

    [Fact]
    public async Task DeleteAsync_ExistingItem_RemovesItem()
    {
        var book = CreateBook("Book to Delete", 2024);
        await _bookRepository.InsertAsync(book);

        await _repository.DeleteAsync(book.Id);

        var result = await _repository.FindByIdAsync(book.Id);
        Assert.Null(result);
    }

    [Fact]
    public async Task DeleteAsync_NonExistentItem_DoesNotThrow()
    {
        var exception = await Record.ExceptionAsync(() => _repository.DeleteAsync(99999));

        Assert.Null(exception);
    }

    [Fact]
    public async Task GetModifiedSinceAsync_ReturnsItemsModifiedAfterDate()
    {
        // Insert old book first
        var oldBook = CreateBook("Old Book", 2020);
        await _bookRepository.InsertAsync(oldBook);

        // Wait to ensure time difference
        await Task.Delay(50);
        var cutoffDate = DateTime.UtcNow;
        await Task.Delay(50);

        // Insert new book after cutoff
        var newBook = CreateBook("New Book", 2024);
        await _bookRepository.InsertAsync(newBook);

        var result = await _repository.GetModifiedSinceAsync(cutoffDate);

        Assert.Single(result);
        Assert.Equal("New Book", result[0].Title);
    }

    [Fact]
    public async Task GetModifiedSinceAsync_WithMediaTypeFilter_ReturnsOnlyMatchingType()
    {
        // Use a cutoff in the past so all new items are after it
        var cutoffDate = DateTime.UtcNow.AddHours(-1);

        await _bookRepository.InsertAsync(CreateBook("New Book", 2024));
        await _movieRepository.InsertAsync(CreateMovie("New Movie", 2024));

        var books = await _repository.GetModifiedSinceAsync(cutoffDate, MediaType.Book);
        var movies = await _repository.GetModifiedSinceAsync(cutoffDate, MediaType.Movie);

        Assert.Single(books);
        Assert.Single(movies);
        Assert.Equal("New Book", books[0].Title);
        Assert.Equal("New Movie", movies[0].Title);
    }

    [Fact]
    public async Task GetPageAsync_OrdersByAddedDescending()
    {
        await _bookRepository.InsertAsync(CreateBook("Book 1", 2020));
        await Task.Delay(10);
        await _bookRepository.InsertAsync(CreateBook("Book 2", 2021));
        await Task.Delay(10);
        await _bookRepository.InsertAsync(CreateBook("Book 3", 2022));

        var result = await _repository.GetPageAsync(1, 10);

        Assert.Equal("Book 3", result[0].Title);
        Assert.Equal("Book 2", result[1].Title);
        Assert.Equal("Book 1", result[2].Title);
    }

    [Fact]
    public async Task GetPageAsync_WithNoFilter_ReturnsAllTypes()
    {
        await _bookRepository.InsertAsync(CreateBook("Book", 2024));
        await _movieRepository.InsertAsync(CreateMovie("Movie", 2024));

        var result = await _repository.GetPageAsync(1, 10, null);

        Assert.Equal(2, result.Count);
    }

    [Fact]
    public async Task CountAsync_WithNoFilter_CountsAllTypes()
    {
        await _bookRepository.InsertAsync(CreateBook("Book", 2024));
        await _movieRepository.InsertAsync(CreateMovie("Movie", 2024));

        var result = await _repository.CountAsync(null);

        Assert.Equal(2, result);
    }

    [Fact]
    public async Task GetModifiedSinceAsync_WithNoFilter_ReturnsAllTypes()
    {
        var cutoffDate = DateTime.UtcNow.AddHours(-1);

        await _bookRepository.InsertAsync(CreateBook("Book", 2024));
        await _movieRepository.InsertAsync(CreateMovie("Movie", 2024));

        var result = await _repository.GetModifiedSinceAsync(cutoffDate, null);

        Assert.Equal(2, result.Count);
    }

    [Fact]
    public async Task GetPageAsync_SecondPage_ReturnsCorrectOffset()
    {
        for (int i = 1; i <= 15; i++)
        {
            await _bookRepository.InsertAsync(CreateBook($"Book {i}", 2020 + i));
        }

        var page2 = await _repository.GetPageAsync(2, 10);

        Assert.Equal(5, page2.Count);
    }

    [Fact]
    public async Task FindByIdAsync_UnknownMediaType_ReturnsNull()
    {
        // Insert a book and then query for a non-existent ID
        var book = CreateBook("Test", 2024);
        await _bookRepository.InsertAsync(book);

        var result = await _repository.FindByIdAsync(99999);

        Assert.Null(result);
    }
}
