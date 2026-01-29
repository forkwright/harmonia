// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Net.Http.Json;
using System.Text;
using System.Text.Json;
using FluentAssertions;
using Mouseion.Api.Authors;
using Mouseion.Api.Books;
using Mouseion.Api.Common;

namespace Mouseion.Api.Tests.Books;

/// <summary>
/// Negative path and edge case tests for BookController.
/// Covers: 401, 403, 404, 409, 422, pagination edge cases, validation boundaries.
/// </summary>
public class BookControllerNegativeTests : ControllerTestBase
{
    public BookControllerNegativeTests(TestWebApplicationFactory factory) : base(factory)
    {
    }

    #region Authorization Tests (401)

    [Fact]
    public async Task GetBooks_WithoutApiKey_Returns401()
    {
        // Arrange
        var client = new HttpClient { BaseAddress = Client.BaseAddress };
        // Intentionally not adding X-Api-Key header

        // Act
        var response = await client.GetAsync("/api/v3/books");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.Unauthorized);
    }

    [Fact]
    public async Task GetBooks_WithInvalidApiKey_Returns401()
    {
        // Arrange
        var client = new HttpClient { BaseAddress = Client.BaseAddress };
        client.DefaultRequestHeaders.Add("X-Api-Key", "invalid-api-key-12345");

        // Act
        var response = await client.GetAsync("/api/v3/books");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.Unauthorized);
    }

    [Fact]
    public async Task AddBook_WithoutApiKey_Returns401()
    {
        // Arrange
        var client = new HttpClient { BaseAddress = Client.BaseAddress };
        var book = new BookResource
        {
            Title = "Test Book",
            Year = 2020,
            Monitored = true,
            QualityProfileId = 1
        };

        // Act
        var response = await client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.Unauthorized);
    }

    [Fact]
    public async Task DeleteBook_WithoutApiKey_Returns401()
    {
        // Arrange
        var client = new HttpClient { BaseAddress = Client.BaseAddress };

        // Act
        var response = await client.DeleteAsync("/api/v3/books/1");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.Unauthorized);
    }

    #endregion

    #region Resource Not Found Tests (404)

    [Fact]
    public async Task GetBook_NonExistent_Returns404()
    {
        // Act
        var response = await Client.GetAsync("/api/v3/books/99999");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.NotFound);
    }

    [Fact]
    public async Task GetBook_WithZeroId_Returns404()
    {
        // Act
        var response = await Client.GetAsync("/api/v3/books/0");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.NotFound);
    }

    [Fact]
    public async Task GetBook_WithNegativeId_Returns404()
    {
        // Act
        var response = await Client.GetAsync("/api/v3/books/-1");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.NotFound);
    }

    [Fact]
    public async Task UpdateBook_NonExistent_Returns404()
    {
        // Arrange
        var book = new BookResource
        {
            Id = 99999,
            Title = "Non-existent Book",
            Year = 2020,
            Monitored = true,
            QualityProfileId = 1
        };

        // Act
        var response = await Client.PutAsJsonAsync("/api/v3/books/99999", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.NotFound);
    }

    [Fact]
    public async Task DeleteBook_NonExistent_Returns404()
    {
        // Act
        var response = await Client.DeleteAsync("/api/v3/books/99999");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.NotFound);
    }

    [Fact]
    public async Task DeleteBook_AlreadyDeleted_Returns404()
    {
        // Arrange - Create and delete a book
        var author = await CreateAuthor("Temp Author");
        var book = await CreateBook("Temp Book", 2020, author.Id);
        await Client.DeleteAsync($"/api/v3/books/{book.Id}");

        // Act - Try to delete again
        var response = await Client.DeleteAsync($"/api/v3/books/{book.Id}");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.NotFound);
    }

    #endregion

    #region Validation Tests (422)

    [Fact]
    public async Task AddBook_WithEmptyTitle_Returns422()
    {
        // Arrange
        var book = new BookResource
        {
            Title = "",
            Year = 2020,
            Monitored = true,
            QualityProfileId = 1
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
        var content = await response.Content.ReadAsStringAsync();
        content.Should().Contain("Title");
    }

    [Fact]
    public async Task AddBook_WithNullTitle_Returns422()
    {
        // Arrange
        var json = """{"title":null,"year":2020,"monitored":true,"qualityProfileId":1}""";
        var content = new StringContent(json, Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v3/books", content);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
    }

    [Fact]
    public async Task AddBook_WithTitleExceeding500Chars_Returns422()
    {
        // Arrange
        var book = new BookResource
        {
            Title = new string('A', 501),
            Year = 2020,
            Monitored = true,
            QualityProfileId = 1
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
        var content = await response.Content.ReadAsStringAsync();
        content.Should().Contain("500");
    }

    [Fact]
    public async Task AddBook_WithYearBelow1000_Returns422()
    {
        // Arrange
        var book = new BookResource
        {
            Title = "Ancient Book",
            Year = 999,
            Monitored = true,
            QualityProfileId = 1
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
        var content = await response.Content.ReadAsStringAsync();
        content.Should().Contain("Year");
    }

    [Fact]
    public async Task AddBook_WithYearAbove2100_Returns422()
    {
        // Arrange
        var book = new BookResource
        {
            Title = "Future Book",
            Year = 2101,
            Monitored = true,
            QualityProfileId = 1
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
        var content = await response.Content.ReadAsStringAsync();
        content.Should().Contain("Year");
    }

    [Fact]
    public async Task AddBook_WithNegativeYear_Returns422()
    {
        // Arrange
        var book = new BookResource
        {
            Title = "Negative Year Book",
            Year = -100,
            Monitored = true,
            QualityProfileId = 1
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
    }

    [Fact]
    public async Task AddBook_WithZeroQualityProfileId_Returns422()
    {
        // Arrange
        var book = new BookResource
        {
            Title = "Valid Title",
            Year = 2020,
            Monitored = true,
            QualityProfileId = 0
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
        var content = await response.Content.ReadAsStringAsync();
        content.Should().Contain("Quality");
    }

    [Fact]
    public async Task AddBook_WithNegativeQualityProfileId_Returns422()
    {
        // Arrange
        var book = new BookResource
        {
            Title = "Valid Title",
            Year = 2020,
            Monitored = true,
            QualityProfileId = -1
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
    }

    [Fact]
    public async Task AddBook_WithNegativeAuthorId_Returns422()
    {
        // Arrange
        var book = new BookResource
        {
            Title = "Valid Title",
            Year = 2020,
            Monitored = true,
            QualityProfileId = 1,
            AuthorId = -1
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
    }

    [Fact]
    public async Task AddBook_WithZeroAuthorId_Returns422()
    {
        // Arrange
        var book = new BookResource
        {
            Title = "Valid Title",
            Year = 2020,
            Monitored = true,
            QualityProfileId = 1,
            AuthorId = 0
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
    }

    [Fact]
    public async Task AddBook_WithDescriptionExceeding5000Chars_Returns422()
    {
        // Arrange
        var book = new BookResource
        {
            Title = "Valid Title",
            Year = 2020,
            Monitored = true,
            QualityProfileId = 1,
            Metadata = new BookMetadataResource
            {
                Description = new string('D', 5001)
            }
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
        var content = await response.Content.ReadAsStringAsync();
        content.Should().Contain("Description");
    }

    [Fact]
    public async Task AddBook_WithNegativePageCount_Returns422()
    {
        // Arrange
        var book = new BookResource
        {
            Title = "Valid Title",
            Year = 2020,
            Monitored = true,
            QualityProfileId = 1,
            Metadata = new BookMetadataResource
            {
                PageCount = -100
            }
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
    }

    [Fact]
    public async Task AddBook_WithZeroPageCount_Returns422()
    {
        // Arrange
        var book = new BookResource
        {
            Title = "Valid Title",
            Year = 2020,
            Monitored = true,
            QualityProfileId = 1,
            Metadata = new BookMetadataResource
            {
                PageCount = 0
            }
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
    }

    [Fact]
    public async Task AddBook_WithIsbnExceeding13Chars_Returns422()
    {
        // Arrange
        var book = new BookResource
        {
            Title = "Valid Title",
            Year = 2020,
            Monitored = true,
            QualityProfileId = 1,
            Metadata = new BookMetadataResource
            {
                Isbn = "12345678901234" // 14 characters
            }
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
    }

    [Fact]
    public async Task AddBook_WithAsinExceeding10Chars_Returns422()
    {
        // Arrange
        var book = new BookResource
        {
            Title = "Valid Title",
            Year = 2020,
            Monitored = true,
            QualityProfileId = 1,
            Metadata = new BookMetadataResource
            {
                Asin = "12345678901" // 11 characters
            }
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
    }

    [Fact]
    public async Task AddBook_WithNegativeSeriesPosition_Returns422()
    {
        // Arrange
        var book = new BookResource
        {
            Title = "Valid Title",
            Year = 2020,
            Monitored = true,
            QualityProfileId = 1,
            Metadata = new BookMetadataResource
            {
                SeriesPosition = -1
            }
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.UnprocessableEntity);
    }

    #endregion

    #region Pagination Edge Cases

    [Fact]
    public async Task GetBooks_WithPageZero_ReturnsPage1()
    {
        // The controller normalizes page=0 to page=1
        // Act
        var response = await Client.GetAsync("/api/v3/books?page=0");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);
        var result = await response.Content.ReadFromJsonAsync<PagedResult<BookResource>>();
        result.Should().NotBeNull();
        result!.Page.Should().Be(1);
    }

    [Fact]
    public async Task GetBooks_WithNegativePage_ReturnsPage1()
    {
        // The controller normalizes negative page to page=1
        // Act
        var response = await Client.GetAsync("/api/v3/books?page=-5");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);
        var result = await response.Content.ReadFromJsonAsync<PagedResult<BookResource>>();
        result.Should().NotBeNull();
        result!.Page.Should().Be(1);
    }

    [Fact]
    public async Task GetBooks_WithPageSizeZero_ReturnsDefaultPageSize()
    {
        // The controller normalizes pageSize=0 to default
        // Act
        var response = await Client.GetAsync("/api/v3/books?pageSize=0");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);
        var result = await response.Content.ReadFromJsonAsync<PagedResult<BookResource>>();
        result.Should().NotBeNull();
        result!.PageSize.Should().Be(50); // Default page size
    }

    [Fact]
    public async Task GetBooks_WithNegativePageSize_ReturnsDefaultPageSize()
    {
        // Act
        var response = await Client.GetAsync("/api/v3/books?pageSize=-10");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);
        var result = await response.Content.ReadFromJsonAsync<PagedResult<BookResource>>();
        result.Should().NotBeNull();
        result!.PageSize.Should().Be(50); // Default page size
    }

    [Fact]
    public async Task GetBooks_WithPageSizeExceedingMax_ReturnsCappedPageSize()
    {
        // Controller caps pageSize at 250
        // Act
        var response = await Client.GetAsync("/api/v3/books?pageSize=10000");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);
        var result = await response.Content.ReadFromJsonAsync<PagedResult<BookResource>>();
        result.Should().NotBeNull();
        result!.PageSize.Should().Be(250);
    }

    [Fact]
    public async Task GetBooks_WithVeryHighPageNumber_ReturnsEmptyResults()
    {
        // Act
        var response = await Client.GetAsync("/api/v3/books?page=999999");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);
        var result = await response.Content.ReadFromJsonAsync<PagedResult<BookResource>>();
        result.Should().NotBeNull();
        result!.Items.Should().BeEmpty();
    }

    [Fact]
    public async Task GetBooks_WithNonNumericPage_Returns400()
    {
        // Act
        var response = await Client.GetAsync("/api/v3/books?page=abc");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.BadRequest);
    }

    [Fact]
    public async Task GetBooks_WithNonNumericPageSize_Returns400()
    {
        // Act
        var response = await Client.GetAsync("/api/v3/books?pageSize=xyz");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.BadRequest);
    }

    #endregion

    #region Relationship Integrity Tests

    [Fact]
    public async Task GetBooksByAuthor_NonExistentAuthor_ReturnsEmptyList()
    {
        // Act
        var response = await Client.GetAsync("/api/v3/books/author/99999");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);
        var books = await response.Content.ReadFromJsonAsync<List<BookResource>>();
        books.Should().NotBeNull();
        books.Should().BeEmpty();
    }

    [Fact]
    public async Task GetBooksBySeries_NonExistentSeries_ReturnsEmptyList()
    {
        // Act
        var response = await Client.GetAsync("/api/v3/books/series/99999");

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);
        var books = await response.Content.ReadFromJsonAsync<List<BookResource>>();
        books.Should().NotBeNull();
        books.Should().BeEmpty();
    }

    #endregion

    #region Malformed Request Tests

    [Fact]
    public async Task AddBook_WithInvalidJson_Returns400()
    {
        // Arrange
        var content = new StringContent("{ invalid json }", Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v3/books", content);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.BadRequest);
    }

    [Fact]
    public async Task AddBook_WithEmptyBody_Returns400()
    {
        // Arrange
        var content = new StringContent("", Encoding.UTF8, "application/json");

        // Act
        var response = await Client.PostAsync("/api/v3/books", content);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.BadRequest);
    }

    [Fact]
    public async Task UpdateBook_WithMismatchedId_ShouldUsePathId()
    {
        // Arrange - Create a book
        var author = await CreateAuthor("Mismatched Author");
        var book = await CreateBook("Original Title", 2020, author.Id);

        // Create update with different ID in body
        var updateResource = new BookResource
        {
            Id = 99999, // Mismatched ID
            Title = "Updated Title",
            Year = 2021,
            Monitored = true,
            QualityProfileId = 1,
            AuthorId = author.Id
        };

        // Act - PUT to the correct path ID
        var response = await Client.PutAsJsonAsync($"/api/v3/books/{book.Id}", updateResource);

        // Assert - Should update the book at path ID
        response.StatusCode.Should().Be(HttpStatusCode.OK);
        var updated = await response.Content.ReadFromJsonAsync<BookResource>();
        updated!.Id.Should().Be(book.Id);
        updated.Title.Should().Be("Updated Title");
    }

    [Fact]
    public async Task AddBooks_Batch_WithEmptyList_ReturnsEmptyList()
    {
        // Arrange
        var books = new List<BookResource>();

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books/batch", books);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.OK);
        var result = await response.Content.ReadFromJsonAsync<List<BookResource>>();
        result.Should().NotBeNull();
        result.Should().BeEmpty();
    }

    #endregion

    #region Boundary Value Tests

    [Fact]
    public async Task AddBook_WithMinimumValidYear_Succeeds()
    {
        // Arrange
        var author = await CreateAuthor("Ancient Author");
        var book = new BookResource
        {
            Title = "Ancient Manuscript",
            Year = 1000, // Minimum valid year
            Monitored = true,
            QualityProfileId = 1,
            AuthorId = author.Id
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.Created);
    }

    [Fact]
    public async Task AddBook_WithMaximumValidYear_Succeeds()
    {
        // Arrange
        var author = await CreateAuthor("Future Author");
        var book = new BookResource
        {
            Title = "Future Publication",
            Year = 2100, // Maximum valid year
            Monitored = true,
            QualityProfileId = 1,
            AuthorId = author.Id
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.Created);
    }

    [Fact]
    public async Task AddBook_WithMaxLength500Title_Succeeds()
    {
        // Arrange
        var author = await CreateAuthor("Long Title Author");
        var book = new BookResource
        {
            Title = new string('A', 500), // Maximum valid length
            Year = 2020,
            Monitored = true,
            QualityProfileId = 1,
            AuthorId = author.Id
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.Created);
    }

    [Fact]
    public async Task AddBook_WithMaxLength5000Description_Succeeds()
    {
        // Arrange
        var author = await CreateAuthor("Long Description Author");
        var book = new BookResource
        {
            Title = "Valid Title",
            Year = 2020,
            Monitored = true,
            QualityProfileId = 1,
            AuthorId = author.Id,
            Metadata = new BookMetadataResource
            {
                Description = new string('D', 5000) // Maximum valid length
            }
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.Created);
    }

    [Fact]
    public async Task AddBook_WithSingleCharacterTitle_Succeeds()
    {
        // Arrange
        var author = await CreateAuthor("Short Title Author");
        var book = new BookResource
        {
            Title = "X", // Minimum valid title
            Year = 2020,
            Monitored = true,
            QualityProfileId = 1,
            AuthorId = author.Id
        };

        // Act
        var response = await Client.PostAsJsonAsync("/api/v3/books", book);

        // Assert
        response.StatusCode.Should().Be(HttpStatusCode.Created);
    }

    #endregion

    #region Helper Methods

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
        return created!;
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
        return created!;
    }

    #endregion
}
