// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation;

namespace Mouseion.Api.Books;

public class BookResourceValidator : AbstractValidator<BookResource>
{
    public BookResourceValidator()
    {
        RuleFor(x => x.Title)
            .NotEmpty().WithMessage("Title is required")
            .MaximumLength(500).WithMessage("Title must not exceed 500 characters");

        RuleFor(x => x.Year)
            .InclusiveBetween(1000, 2100).WithMessage("Year must be between 1000 and 2100");

        RuleFor(x => x.QualityProfileId)
            .GreaterThan(0).WithMessage("Quality profile ID must be greater than 0");

        RuleFor(x => x.AuthorId)
            .GreaterThan(0).When(x => x.AuthorId.HasValue)
            .WithMessage("Author ID must be greater than 0 when provided");

        RuleFor(x => x.BookSeriesId)
            .GreaterThan(0).When(x => x.BookSeriesId.HasValue)
            .WithMessage("Book series ID must be greater than 0 when provided");

        RuleFor(x => x.Metadata)
            .NotNull().WithMessage("Metadata is required")
            .SetValidator(new BookMetadataResourceValidator());
    }
}

public class BookMetadataResourceValidator : AbstractValidator<BookMetadataResource>
{
    public BookMetadataResourceValidator()
    {
        RuleFor(x => x.Description)
            .MaximumLength(5000).When(x => !string.IsNullOrEmpty(x.Description))
            .WithMessage("Description must not exceed 5000 characters");

        RuleFor(x => x.ForeignBookId)
            .MaximumLength(200).When(x => !string.IsNullOrEmpty(x.ForeignBookId))
            .WithMessage("Foreign book ID must not exceed 200 characters");

        RuleFor(x => x.GoodreadsId)
            .MaximumLength(100).When(x => !string.IsNullOrEmpty(x.GoodreadsId))
            .WithMessage("Goodreads ID must not exceed 100 characters");

        RuleFor(x => x.OpenLibraryId)
            .MaximumLength(100).When(x => !string.IsNullOrEmpty(x.OpenLibraryId))
            .WithMessage("OpenLibrary ID must not exceed 100 characters");

        RuleFor(x => x.GoogleBooksId)
            .MaximumLength(100).When(x => !string.IsNullOrEmpty(x.GoogleBooksId))
            .WithMessage("Google Books ID must not exceed 100 characters");

        RuleFor(x => x.Isbn)
            .MaximumLength(13).When(x => !string.IsNullOrEmpty(x.Isbn))
            .WithMessage("ISBN must not exceed 13 characters");

        RuleFor(x => x.Isbn13)
            .MaximumLength(13).When(x => !string.IsNullOrEmpty(x.Isbn13))
            .WithMessage("ISBN-13 must not exceed 13 characters");

        RuleFor(x => x.Asin)
            .MaximumLength(10).When(x => !string.IsNullOrEmpty(x.Asin))
            .WithMessage("ASIN must not exceed 10 characters");

        RuleFor(x => x.PageCount)
            .GreaterThan(0).When(x => x.PageCount.HasValue)
            .WithMessage("Page count must be greater than 0 when provided");

        RuleFor(x => x.Publisher)
            .MaximumLength(200).When(x => !string.IsNullOrEmpty(x.Publisher))
            .WithMessage("Publisher must not exceed 200 characters");

        RuleFor(x => x.Language)
            .MaximumLength(50).When(x => !string.IsNullOrEmpty(x.Language))
            .WithMessage("Language must not exceed 50 characters");

        RuleFor(x => x.SeriesPosition)
            .GreaterThan(0).When(x => x.SeriesPosition.HasValue)
            .WithMessage("Series position must be greater than 0 when provided");
    }
}
