// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using Mouseion.Api.Authors;
using Mouseion.Api.Books;
using Mouseion.Api.Common;

namespace Mouseion.Api.Tests.Books;

public class BookControllerTests : ControllerTestBase
{
    public BookControllerTests(TestWebApplicationFactory factory) : base(factory)
    {
    }

    [Fact]
    public async Task GetBooks_ReturnsSuccessfully()
    {
        var response = await Client.GetAsync("/api/v3/books");
        response.EnsureSuccessStatusCode();

        var result = await response.Content.ReadFromJsonAsync<PagedResult<BookResource>>();
        Assert.NotNull(result);
        Assert.NotNull(result.Items);
    }

    [Fact]
    public async Task AddBook_ReturnsCreated_WithValidBook()
    {
        var author = await CreateAuthor("Brandon Sanderson");

        var book = new BookResource
        {
            Title = "The Way of Kings",
            Year = 2010,
            Monitored = true,
            QualityProfileId = 1,
            AuthorId = author.Id,
            Metadata = new BookMetadataResource
            {
                Description = "Epic fantasy novel",
                Isbn13 = "9780765326355",
                PageCount = 1007,
                Publisher = "Tor Books"
            }
        };

        var response = await Client.PostAsJsonAsync("/api/v3/books", book);
        Assert.Equal(HttpStatusCode.Created, response.StatusCode);

        var created = await response.Content.ReadFromJsonAsync<BookResource>();
        Assert.NotNull(created);
        Assert.True(created.Id > 0);
        Assert.Equal("The Way of Kings", created.Title);
        Assert.Equal(2010, created.Year);
        Assert.Equal(author.Id, created.AuthorId);
        Assert.Equal(1007, created.Metadata.PageCount);
    }

    [Fact]
    public async Task GetBooksByAuthor_ReturnsBooks_ForExistingAuthor()
    {
        var author = await CreateAuthor("Patrick Rothfuss");

        var book1 = await CreateBook("The Name of the Wind", 2007, author.Id);
        var book2 = await CreateBook("The Wise Man's Fear", 2011, author.Id);

        var response = await Client.GetAsync($"/api/v3/books/author/{author.Id}");
        response.EnsureSuccessStatusCode();

        var books = await response.Content.ReadFromJsonAsync<List<BookResource>>();
        Assert.NotNull(books);
        Assert.Equal(2, books.Count);
        Assert.Contains(books, b => b.Title == "The Name of the Wind");
        Assert.Contains(books, b => b.Title == "The Wise Man's Fear");
    }

    [Fact]
    public async Task GetStatistics_ReturnsAggregatedData()
    {
        var author = await CreateAuthor("Joe Abercrombie");
        await CreateBook("The Blade Itself", 2006, author.Id);
        await CreateBook("Before They Are Hanged", 2007, author.Id);

        var response = await Client.GetAsync("/api/v3/books/statistics");
        response.EnsureSuccessStatusCode();

        var stats = await response.Content.ReadFromJsonAsync<dynamic>();
        Assert.NotNull(stats);
    }

    [Fact]
    public async Task UpdateBook_UpdatesFields_WhenValid()
    {
        var author = await CreateAuthor("N.K. Jemisin");
        var book = await CreateBook("The Fifth Season", 2015, author.Id);

        book.Monitored = false;
        book.Metadata.Description = "Award-winning science fantasy";

        var updateResponse = await Client.PutAsJsonAsync($"/api/v3/books/{book.Id}", book);
        updateResponse.EnsureSuccessStatusCode();

        var updated = await updateResponse.Content.ReadFromJsonAsync<BookResource>();
        Assert.NotNull(updated);
        Assert.False(updated.Monitored);
        Assert.Equal("Award-winning science fantasy", updated.Metadata.Description);
    }

    [Fact]
    public async Task DeleteBook_ReturnsNoContent_WhenExists()
    {
        var author = await CreateAuthor("Ursula K. Le Guin");
        var book = await CreateBook("The Left Hand of Darkness", 1969, author.Id);

        var deleteResponse = await Client.DeleteAsync($"/api/v3/books/{book.Id}");
        Assert.Equal(HttpStatusCode.NoContent, deleteResponse.StatusCode);

        var getResponse = await Client.GetAsync($"/api/v3/books/{book.Id}");
        Assert.Equal(HttpStatusCode.NotFound, getResponse.StatusCode);
    }

    private async Task<AuthorResource> CreateAuthor(string name)
    {
        var author = new AuthorResource
        {
            Name = name,
            Monitored = true,
            QualityProfileId = 1
        };

        var response = await Client.PostAsJsonAsync("/api/v3/authors", author);
        response.EnsureSuccessStatusCode();

        var created = await response.Content.ReadFromJsonAsync<AuthorResource>();
        Assert.NotNull(created);
        return created;
    }

    private async Task<BookResource> CreateBook(string title, int year, int authorId)
    {
        var book = new BookResource
        {
            Title = title,
            Year = year,
            Monitored = true,
            QualityProfileId = 1,
            AuthorId = authorId
        };

        var response = await Client.PostAsJsonAsync("/api/v3/books", book);
        response.EnsureSuccessStatusCode();

        var created = await response.Content.ReadFromJsonAsync<BookResource>();
        Assert.NotNull(created);
        return created;
    }
}
