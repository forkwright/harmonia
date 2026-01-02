// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Authors;
using Mouseion.Core.MediaItems;

namespace Mouseion.Core.Books;

public interface IAddBookService
{
    Task<Book> AddBookAsync(Book book, CancellationToken ct = default);
    Task<List<Book>> AddBooksAsync(List<Book> books, CancellationToken ct = default);

    Book AddBook(Book book);
    List<Book> AddBooks(List<Book> books);
}

public class AddBookService : AddMediaItemService<Book, IBookRepository>, IAddBookService
{
    private readonly IBookRepository _bookRepository;

    public AddBookService(
        IBookRepository bookRepository,
        IAuthorRepository authorRepository,
        ILogger<AddBookService> logger)
        : base(bookRepository, authorRepository, logger)
    {
        _bookRepository = bookRepository;
    }

    public Task<Book> AddBookAsync(Book book, CancellationToken ct = default)
        => AddItemAsync(book, ct);

    public Book AddBook(Book book)
        => AddItem(book);

    public Task<List<Book>> AddBooksAsync(List<Book> books, CancellationToken ct = default)
        => AddItemsAsync(books, ct);

    public List<Book> AddBooks(List<Book> books)
        => AddItems(books);

    protected override Task<Book?> FindByTitleAsync(string title, int year, CancellationToken ct = default)
        => _bookRepository.FindByTitleAsync(title, year, ct);

    protected override Book? FindByTitle(string title, int year)
        => _bookRepository.FindByTitle(title, year);

    protected override void LogItemAdded(Book book)
        => Logger.LogInformation("Added book: {BookTitle} ({Year}) - Author ID: {AuthorId}",
            book.Title, book.Year, book.AuthorId);

    protected override void LogItemExists(Book book)
        => Logger.LogInformation("Book already exists: {BookTitle} ({Year})", book.Title, book.Year);
}
