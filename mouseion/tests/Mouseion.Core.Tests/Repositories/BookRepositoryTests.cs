// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Books;

namespace Mouseion.Core.Tests.Repositories;

public class BookRepositoryTests : RepositoryTestBase
{
    private readonly IBookRepository _repository;

    public BookRepositoryTests()
    {
        _repository = new BookRepository(Database);
    }

    [Fact]
    public async Task AllAsync_ReturnsOnlyBooks()
    {
        var book1 = CreateBook("Book 1", 2023);
        var book2 = CreateBook("Book 2", 2024);
        await _repository.InsertAsync(book1);
        await _repository.InsertAsync(book2);

        var result = await _repository.AllAsync();

        Assert.Equal(2, result.Count());
        Assert.All(result, b => Assert.NotNull(b.Title));
    }

    [Fact]
    public void All_ReturnsOnlyBooks()
    {
        var book1 = CreateBook("Book 1", 2023);
        var book2 = CreateBook("Book 2", 2024);
        _repository.Insert(book1);
        _repository.Insert(book2);

        var result = _repository.All();

        Assert.Equal(2, result.Count());
        Assert.All(result, b => Assert.NotNull(b.Title));
    }

    [Fact]
    public async Task GetPageAsync_ReturnsPaginatedResults()
    {
        for (int i = 1; i <= 15; i++)
        {
            await _repository.InsertAsync(CreateBook($"Book {i}", 2020 + i));
        }

        var page1 = await _repository.GetPageAsync(1, 5);
        var page2 = await _repository.GetPageAsync(2, 5);

        Assert.Equal(5, page1.Count());
        Assert.Equal(5, page2.Count());
        Assert.NotEqual(page1.First().Id, page2.First().Id);
    }

    [Fact]
    public async Task FindAsync_ReturnsBookById()
    {
        var book = CreateBook("Findable Book", 2024);
        var inserted = await _repository.InsertAsync(book);

        var found = await _repository.FindAsync(inserted.Id);

        Assert.NotNull(found);
        Assert.Equal("Findable Book", found.Title);
        Assert.Equal(2024, found.Year);
    }

    [Fact]
    public async Task FindAsync_ReturnsNullWhenNotFound()
    {
        var found = await _repository.FindAsync(999);

        Assert.Null(found);
    }

    [Fact]
    public void Find_ReturnsBookById()
    {
        var book = CreateBook("Findable Book", 2024);
        var inserted = _repository.Insert(book);

        var found = _repository.Find(inserted.Id);

        Assert.NotNull(found);
        Assert.Equal("Findable Book", found.Title);
    }

    [Fact]
    public async Task FindByTitleAsync_ReturnsMatchingBook()
    {
        var book = CreateBook("Unique Title", 2024);
        await _repository.InsertAsync(book);

        var found = await _repository.FindByTitleAsync("Unique Title", 2024);

        Assert.NotNull(found);
        Assert.Equal("Unique Title", found.Title);
        Assert.Equal(2024, found.Year);
    }

    [Fact]
    public async Task FindByTitleAsync_ReturnsNullWhenNoMatch()
    {
        var found = await _repository.FindByTitleAsync("Nonexistent Book", 2024);

        Assert.Null(found);
    }

    [Fact]
    public void FindByTitle_ReturnsMatchingBook()
    {
        var book = CreateBook("Unique Title", 2024);
        _repository.Insert(book);

        var found = _repository.FindByTitle("Unique Title", 2024);

        Assert.NotNull(found);
        Assert.Equal("Unique Title", found.Title);
    }

    [Fact]
    public async Task GetByAuthorIdAsync_ReturnsBooksByAuthor()
    {
        var book1 = CreateBook("Author 1 Book 1", 2023, authorId: 1);
        var book2 = CreateBook("Author 1 Book 2", 2024, authorId: 1);
        var book3 = CreateBook("Author 2 Book 1", 2024, authorId: 2);

        await _repository.InsertAsync(book1);
        await _repository.InsertAsync(book2);
        await _repository.InsertAsync(book3);

        var author1Books = await _repository.GetByAuthorIdAsync(1);

        Assert.Equal(2, author1Books.Count);
        Assert.All(author1Books, b => Assert.Equal(1, b.AuthorId));
    }

    [Fact]
    public void GetByAuthorId_ReturnsBooksByAuthor()
    {
        var book1 = CreateBook("Author 1 Book 1", 2023, authorId: 1);
        var book2 = CreateBook("Author 1 Book 2", 2024, authorId: 1);

        _repository.Insert(book1);
        _repository.Insert(book2);

        var author1Books = _repository.GetByAuthorId(1);

        Assert.Equal(2, author1Books.Count);
    }

    [Fact]
    public async Task GetBySeriesIdAsync_ReturnsBooksBySeries()
    {
        var book1 = CreateBook("Series 1 Book 1", 2023, bookSeriesId: 1);
        var book2 = CreateBook("Series 1 Book 2", 2024, bookSeriesId: 1);
        var book3 = CreateBook("Series 2 Book 1", 2024, bookSeriesId: 2);

        await _repository.InsertAsync(book1);
        await _repository.InsertAsync(book2);
        await _repository.InsertAsync(book3);

        var series1Books = await _repository.GetBySeriesIdAsync(1);

        Assert.Equal(2, series1Books.Count);
        Assert.All(series1Books, b => Assert.Equal(1, b.BookSeriesId));
    }

    [Fact]
    public void GetBySeriesId_ReturnsBooksBySeries()
    {
        var book1 = CreateBook("Series 1 Book 1", 2023, bookSeriesId: 1);
        var book2 = CreateBook("Series 1 Book 2", 2024, bookSeriesId: 1);

        _repository.Insert(book1);
        _repository.Insert(book2);

        var series1Books = _repository.GetBySeriesId(1);

        Assert.Equal(2, series1Books.Count);
    }

    [Fact]
    public async Task GetMonitoredAsync_ReturnsOnlyMonitoredBooks()
    {
        var monitored1 = CreateBook("Monitored 1", 2023, monitored: true);
        var monitored2 = CreateBook("Monitored 2", 2024, monitored: true);
        var unmonitored = CreateBook("Unmonitored", 2024, monitored: false);

        await _repository.InsertAsync(monitored1);
        await _repository.InsertAsync(monitored2);
        await _repository.InsertAsync(unmonitored);

        var monitoredBooks = await _repository.GetMonitoredAsync();

        Assert.Equal(2, monitoredBooks.Count);
        Assert.All(monitoredBooks, b => Assert.True(b.Monitored));
    }

    [Fact]
    public void GetMonitored_ReturnsOnlyMonitoredBooks()
    {
        var monitored1 = CreateBook("Monitored 1", 2023, monitored: true);
        var unmonitored = CreateBook("Unmonitored", 2024, monitored: false);

        _repository.Insert(monitored1);
        _repository.Insert(unmonitored);

        var monitoredBooks = _repository.GetMonitored();

        Assert.Single(monitoredBooks);
        Assert.True(monitoredBooks[0].Monitored);
    }

    [Fact]
    public async Task BookExistsAsync_ReturnsTrueWhenExists()
    {
        var book = CreateBook("Existing Book", 2024, authorId: 1);
        await _repository.InsertAsync(book);

        var exists = await _repository.BookExistsAsync(1, "Existing Book", 2024);

        Assert.True(exists);
    }

    [Fact]
    public async Task BookExistsAsync_ReturnsFalseWhenNotExists()
    {
        var exists = await _repository.BookExistsAsync(1, "Nonexistent Book", 2024);

        Assert.False(exists);
    }

    [Fact]
    public void BookExists_ReturnsTrueWhenExists()
    {
        var book = CreateBook("Existing Book", 2024, authorId: 1);
        _repository.Insert(book);

        var exists = _repository.BookExists(1, "Existing Book", 2024);

        Assert.True(exists);
    }

    [Fact]
    public async Task UpdateAsync_ModifiesBook()
    {
        var book = CreateBook("Original Title", 2024);
        var inserted = await _repository.InsertAsync(book);

        inserted.Title = "Updated Title";
        await _repository.UpdateAsync(inserted);

        var updated = await _repository.FindAsync(inserted.Id);
        Assert.Equal("Updated Title", updated!.Title);
    }

    [Fact]
    public async Task DeleteAsync_RemovesBook()
    {
        var book = CreateBook("To Delete", 2024);
        var inserted = await _repository.InsertAsync(book);

        await _repository.DeleteAsync(inserted.Id);

        var deleted = await _repository.FindAsync(inserted.Id);
        Assert.Null(deleted);
    }

    [Fact]
    public async Task CountAsync_ReturnsCorrectCount()
    {
        await _repository.InsertAsync(CreateBook("Book 1", 2023));
        await _repository.InsertAsync(CreateBook("Book 2", 2024));

        var count = await _repository.CountAsync();

        Assert.Equal(2, count);
    }
}
