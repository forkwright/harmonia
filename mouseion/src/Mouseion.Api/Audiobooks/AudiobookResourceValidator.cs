// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation;

namespace Mouseion.Api.Audiobooks;

public class AudiobookResourceValidator : AbstractValidator<AudiobookResource>
{
    public AudiobookResourceValidator()
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
            .SetValidator(new AudiobookMetadataResourceValidator());
    }
}

public class AudiobookMetadataResourceValidator : AbstractValidator<AudiobookMetadataResource>
{
    public AudiobookMetadataResourceValidator()
    {
        RuleFor(x => x.Description)
            .MaximumLength(5000).When(x => !string.IsNullOrEmpty(x.Description))
            .WithMessage("Description must not exceed 5000 characters");

        RuleFor(x => x.ForeignAudiobookId)
            .MaximumLength(200).When(x => !string.IsNullOrEmpty(x.ForeignAudiobookId))
            .WithMessage("Foreign audiobook ID must not exceed 200 characters");

        RuleFor(x => x.AudnexusId)
            .MaximumLength(100).When(x => !string.IsNullOrEmpty(x.AudnexusId))
            .WithMessage("Audnexus ID must not exceed 100 characters");

        RuleFor(x => x.AudibleId)
            .MaximumLength(100).When(x => !string.IsNullOrEmpty(x.AudibleId))
            .WithMessage("Audible ID must not exceed 100 characters");

        RuleFor(x => x.Isbn)
            .MaximumLength(13).When(x => !string.IsNullOrEmpty(x.Isbn))
            .WithMessage("ISBN must not exceed 13 characters");

        RuleFor(x => x.Isbn13)
            .MaximumLength(13).When(x => !string.IsNullOrEmpty(x.Isbn13))
            .WithMessage("ISBN-13 must not exceed 13 characters");

        RuleFor(x => x.Asin)
            .MaximumLength(10).When(x => !string.IsNullOrEmpty(x.Asin))
            .WithMessage("ASIN must not exceed 10 characters");

        RuleFor(x => x.Publisher)
            .MaximumLength(200).When(x => !string.IsNullOrEmpty(x.Publisher))
            .WithMessage("Publisher must not exceed 200 characters");

        RuleFor(x => x.Language)
            .MaximumLength(50).When(x => !string.IsNullOrEmpty(x.Language))
            .WithMessage("Language must not exceed 50 characters");

        RuleFor(x => x.Narrator)
            .MaximumLength(200).When(x => !string.IsNullOrEmpty(x.Narrator))
            .WithMessage("Narrator must not exceed 200 characters");

        RuleFor(x => x.DurationMinutes)
            .GreaterThan(0).When(x => x.DurationMinutes.HasValue)
            .WithMessage("Duration must be greater than 0 when provided");

        RuleFor(x => x.SeriesPosition)
            .GreaterThan(0).When(x => x.SeriesPosition.HasValue)
            .WithMessage("Series position must be greater than 0 when provided");

        RuleFor(x => x.BookId)
            .GreaterThan(0).When(x => x.BookId.HasValue)
            .WithMessage("Book ID must be greater than 0 when provided");
    }
}
