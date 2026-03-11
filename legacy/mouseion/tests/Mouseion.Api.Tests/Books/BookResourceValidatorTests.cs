// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation.TestHelper;
using Mouseion.Api.Books;
using Xunit;

namespace Mouseion.Api.Tests.Books;

/// <summary>
/// Unit tests for BookResourceValidator covering all validation rules.
/// </summary>
public class BookResourceValidatorTests
{
    private readonly BookResourceValidator _validator = new();

    #region Title Validation

    [Fact]
    public void Validate_WithValidTitle_ShouldPass()
    {
        var resource = CreateValidBookResource();
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.Title);
    }

    [Fact]
    public void Validate_WithEmptyTitle_ShouldFail()
    {
        var resource = CreateValidBookResource();
        resource.Title = "";

        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.Title)
            .WithErrorMessage("Title is required");
    }

    [Fact]
    public void Validate_WithNullTitle_ShouldFail()
    {
        var resource = CreateValidBookResource();
        resource.Title = null!;

        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.Title);
    }

    [Fact]
    public void Validate_WithWhitespaceOnlyTitle_ShouldFail()
    {
        var resource = CreateValidBookResource();
        resource.Title = "   ";

        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.Title);
    }

    [Theory]
    [InlineData(1)]
    [InlineData(100)]
    [InlineData(499)]
    [InlineData(500)]
    public void Validate_WithTitleWithinLimit_ShouldPass(int length)
    {
        var resource = CreateValidBookResource();
        resource.Title = new string('A', length);

        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.Title);
    }

    [Theory]
    [InlineData(501)]
    [InlineData(1000)]
    [InlineData(5000)]
    public void Validate_WithTitleExceedingLimit_ShouldFail(int length)
    {
        var resource = CreateValidBookResource();
        resource.Title = new string('A', length);

        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.Title)
            .WithErrorMessage("Title must not exceed 500 characters");
    }

    #endregion

    #region Year Validation

    [Theory]
    [InlineData(1000)]
    [InlineData(1500)]
    [InlineData(2000)]
    [InlineData(2024)]
    [InlineData(2100)]
    public void Validate_WithValidYear_ShouldPass(int year)
    {
        var resource = CreateValidBookResource();
        resource.Year = year;

        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.Year);
    }

    [Theory]
    [InlineData(999)]
    [InlineData(0)]
    [InlineData(-100)]
    [InlineData(-1)]
    public void Validate_WithYearBelowMinimum_ShouldFail(int year)
    {
        var resource = CreateValidBookResource();
        resource.Year = year;

        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.Year)
            .WithErrorMessage("Year must be between 1000 and 2100");
    }

    [Theory]
    [InlineData(2101)]
    [InlineData(3000)]
    [InlineData(9999)]
    public void Validate_WithYearAboveMaximum_ShouldFail(int year)
    {
        var resource = CreateValidBookResource();
        resource.Year = year;

        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.Year)
            .WithErrorMessage("Year must be between 1000 and 2100");
    }

    #endregion

    #region QualityProfileId Validation

    [Theory]
    [InlineData(1)]
    [InlineData(10)]
    [InlineData(100)]
    public void Validate_WithValidQualityProfileId_ShouldPass(int id)
    {
        var resource = CreateValidBookResource();
        resource.QualityProfileId = id;

        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.QualityProfileId);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(-1)]
    [InlineData(-100)]
    public void Validate_WithInvalidQualityProfileId_ShouldFail(int id)
    {
        var resource = CreateValidBookResource();
        resource.QualityProfileId = id;

        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.QualityProfileId)
            .WithErrorMessage("Quality profile ID must be greater than 0");
    }

    #endregion

    #region AuthorId Validation

    [Fact]
    public void Validate_WithNullAuthorId_ShouldPass()
    {
        var resource = CreateValidBookResource();
        resource.AuthorId = null;

        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.AuthorId);
    }

    [Theory]
    [InlineData(1)]
    [InlineData(100)]
    [InlineData(1000)]
    public void Validate_WithValidAuthorId_ShouldPass(int id)
    {
        var resource = CreateValidBookResource();
        resource.AuthorId = id;

        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.AuthorId);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(-1)]
    [InlineData(-100)]
    public void Validate_WithInvalidAuthorId_ShouldFail(int id)
    {
        var resource = CreateValidBookResource();
        resource.AuthorId = id;

        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.AuthorId)
            .WithErrorMessage("Author ID must be greater than 0 when provided");
    }

    #endregion

    #region BookSeriesId Validation

    [Fact]
    public void Validate_WithNullBookSeriesId_ShouldPass()
    {
        var resource = CreateValidBookResource();
        resource.BookSeriesId = null;

        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.BookSeriesId);
    }

    [Theory]
    [InlineData(1)]
    [InlineData(50)]
    public void Validate_WithValidBookSeriesId_ShouldPass(int id)
    {
        var resource = CreateValidBookResource();
        resource.BookSeriesId = id;

        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.BookSeriesId);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(-1)]
    public void Validate_WithInvalidBookSeriesId_ShouldFail(int id)
    {
        var resource = CreateValidBookResource();
        resource.BookSeriesId = id;

        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.BookSeriesId)
            .WithErrorMessage("Book series ID must be greater than 0 when provided");
    }

    #endregion

    #region Metadata Validation

    [Fact]
    public void Validate_WithNullMetadata_ShouldFail()
    {
        var resource = CreateValidBookResource();
        resource.Metadata = null!;

        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.Metadata)
            .WithErrorMessage("Metadata is required");
    }

    [Fact]
    public void Validate_WithEmptyMetadata_ShouldPass()
    {
        var resource = CreateValidBookResource();
        resource.Metadata = new BookMetadataResource();

        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.Metadata);
    }

    #endregion

    #region Helper Methods

    private static BookResource CreateValidBookResource()
    {
        return new BookResource
        {
            Title = "Valid Book Title",
            Year = 2020,
            QualityProfileId = 1,
            Monitored = true,
            Metadata = new BookMetadataResource()
        };
    }

    #endregion
}

/// <summary>
/// Unit tests for BookMetadataResourceValidator covering all validation rules.
/// </summary>
public class BookMetadataResourceValidatorTests
{
    private readonly BookMetadataResourceValidator _validator = new();

    #region Description Validation

    [Fact]
    public void Validate_WithNullDescription_ShouldPass()
    {
        var resource = new BookMetadataResource { Description = null };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.Description);
    }

    [Fact]
    public void Validate_WithEmptyDescription_ShouldPass()
    {
        var resource = new BookMetadataResource { Description = "" };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.Description);
    }

    [Theory]
    [InlineData(1)]
    [InlineData(1000)]
    [InlineData(5000)]
    public void Validate_WithDescriptionWithinLimit_ShouldPass(int length)
    {
        var resource = new BookMetadataResource { Description = new string('D', length) };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.Description);
    }

    [Theory]
    [InlineData(5001)]
    [InlineData(10000)]
    public void Validate_WithDescriptionExceedingLimit_ShouldFail(int length)
    {
        var resource = new BookMetadataResource { Description = new string('D', length) };
        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.Description)
            .WithErrorMessage("Description must not exceed 5000 characters");
    }

    #endregion

    #region ISBN Validation

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("1234567890")]
    [InlineData("1234567890123")] // 13 chars - valid ISBN-13
    public void Validate_WithValidIsbn_ShouldPass(string? isbn)
    {
        var resource = new BookMetadataResource { Isbn = isbn };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.Isbn);
    }

    [Fact]
    public void Validate_WithIsbnExceedingLimit_ShouldFail()
    {
        var resource = new BookMetadataResource { Isbn = "12345678901234" }; // 14 chars
        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.Isbn)
            .WithErrorMessage("ISBN must not exceed 13 characters");
    }

    #endregion

    #region ISBN-13 Validation

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("9780765326355")] // Valid ISBN-13
    public void Validate_WithValidIsbn13_ShouldPass(string? isbn13)
    {
        var resource = new BookMetadataResource { Isbn13 = isbn13 };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.Isbn13);
    }

    [Fact]
    public void Validate_WithIsbn13ExceedingLimit_ShouldFail()
    {
        var resource = new BookMetadataResource { Isbn13 = "97807653263551" }; // 14 chars
        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.Isbn13)
            .WithErrorMessage("ISBN-13 must not exceed 13 characters");
    }

    #endregion

    #region ASIN Validation

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("B00ZQB4Y9Q")] // Valid 10-char ASIN
    public void Validate_WithValidAsin_ShouldPass(string? asin)
    {
        var resource = new BookMetadataResource { Asin = asin };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.Asin);
    }

    [Fact]
    public void Validate_WithAsinExceedingLimit_ShouldFail()
    {
        var resource = new BookMetadataResource { Asin = "B00ZQB4Y9Q1" }; // 11 chars
        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.Asin)
            .WithErrorMessage("ASIN must not exceed 10 characters");
    }

    #endregion

    #region PageCount Validation

    [Fact]
    public void Validate_WithNullPageCount_ShouldPass()
    {
        var resource = new BookMetadataResource { PageCount = null };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.PageCount);
    }

    [Theory]
    [InlineData(1)]
    [InlineData(100)]
    [InlineData(1000)]
    [InlineData(10000)]
    public void Validate_WithValidPageCount_ShouldPass(int pageCount)
    {
        var resource = new BookMetadataResource { PageCount = pageCount };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.PageCount);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(-1)]
    [InlineData(-100)]
    public void Validate_WithInvalidPageCount_ShouldFail(int pageCount)
    {
        var resource = new BookMetadataResource { PageCount = pageCount };
        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.PageCount)
            .WithErrorMessage("Page count must be greater than 0 when provided");
    }

    #endregion

    #region Publisher Validation

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("Tor Books")]
    public void Validate_WithValidPublisher_ShouldPass(string? publisher)
    {
        var resource = new BookMetadataResource { Publisher = publisher };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.Publisher);
    }

    [Fact]
    public void Validate_WithPublisherAtLimit_ShouldPass()
    {
        var resource = new BookMetadataResource { Publisher = new string('P', 200) };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.Publisher);
    }

    [Fact]
    public void Validate_WithPublisherExceedingLimit_ShouldFail()
    {
        var resource = new BookMetadataResource { Publisher = new string('P', 201) };
        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.Publisher)
            .WithErrorMessage("Publisher must not exceed 200 characters");
    }

    #endregion

    #region Language Validation

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("en")]
    [InlineData("en-US")]
    [InlineData("English")]
    public void Validate_WithValidLanguage_ShouldPass(string? language)
    {
        var resource = new BookMetadataResource { Language = language };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.Language);
    }

    [Fact]
    public void Validate_WithLanguageAtLimit_ShouldPass()
    {
        var resource = new BookMetadataResource { Language = new string('L', 50) };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.Language);
    }

    [Fact]
    public void Validate_WithLanguageExceedingLimit_ShouldFail()
    {
        var resource = new BookMetadataResource { Language = new string('L', 51) };
        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.Language)
            .WithErrorMessage("Language must not exceed 50 characters");
    }

    #endregion

    #region SeriesPosition Validation

    [Fact]
    public void Validate_WithNullSeriesPosition_ShouldPass()
    {
        var resource = new BookMetadataResource { SeriesPosition = null };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.SeriesPosition);
    }

    [Theory]
    [InlineData(1)]
    [InlineData(10)]
    [InlineData(100)]
    public void Validate_WithValidSeriesPosition_ShouldPass(int position)
    {
        var resource = new BookMetadataResource { SeriesPosition = position };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.SeriesPosition);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(-1)]
    [InlineData(-100)]
    public void Validate_WithInvalidSeriesPosition_ShouldFail(int position)
    {
        var resource = new BookMetadataResource { SeriesPosition = position };
        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.SeriesPosition)
            .WithErrorMessage("Series position must be greater than 0 when provided");
    }

    #endregion

    #region External ID Validations

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("12345")]
    public void Validate_WithValidForeignBookId_ShouldPass(string? foreignBookId)
    {
        var resource = new BookMetadataResource { ForeignBookId = foreignBookId };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.ForeignBookId);
    }

    [Fact]
    public void Validate_WithForeignBookIdExceedingLimit_ShouldFail()
    {
        var resource = new BookMetadataResource { ForeignBookId = new string('F', 201) };
        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.ForeignBookId)
            .WithErrorMessage("Foreign book ID must not exceed 200 characters");
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("OL12345M")]
    public void Validate_WithValidOpenLibraryId_ShouldPass(string? openLibraryId)
    {
        var resource = new BookMetadataResource { OpenLibraryId = openLibraryId };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.OpenLibraryId);
    }

    [Fact]
    public void Validate_WithOpenLibraryIdExceedingLimit_ShouldFail()
    {
        var resource = new BookMetadataResource { OpenLibraryId = new string('O', 101) };
        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.OpenLibraryId)
            .WithErrorMessage("OpenLibrary ID must not exceed 100 characters");
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("12345678")]
    public void Validate_WithValidGoodreadsId_ShouldPass(string? goodreadsId)
    {
        var resource = new BookMetadataResource { GoodreadsId = goodreadsId };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.GoodreadsId);
    }

    [Fact]
    public void Validate_WithGoodreadsIdExceedingLimit_ShouldFail()
    {
        var resource = new BookMetadataResource { GoodreadsId = new string('G', 101) };
        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.GoodreadsId)
            .WithErrorMessage("Goodreads ID must not exceed 100 characters");
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    [InlineData("abcd1234")]
    public void Validate_WithValidGoogleBooksId_ShouldPass(string? googleBooksId)
    {
        var resource = new BookMetadataResource { GoogleBooksId = googleBooksId };
        var result = _validator.TestValidate(resource);
        result.ShouldNotHaveValidationErrorFor(x => x.GoogleBooksId);
    }

    [Fact]
    public void Validate_WithGoogleBooksIdExceedingLimit_ShouldFail()
    {
        var resource = new BookMetadataResource { GoogleBooksId = new string('G', 101) };
        var result = _validator.TestValidate(resource);
        result.ShouldHaveValidationErrorFor(x => x.GoogleBooksId)
            .WithErrorMessage("Google Books ID must not exceed 100 characters");
    }

    #endregion
}
