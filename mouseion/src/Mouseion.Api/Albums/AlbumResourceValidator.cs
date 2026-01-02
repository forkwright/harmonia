// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation;

namespace Mouseion.Api.Albums;

public class AlbumResourceValidator : AbstractValidator<AlbumResource>
{
    public AlbumResourceValidator()
    {
        RuleFor(x => x.Title)
            .NotEmpty().WithMessage("Title is required")
            .MaximumLength(500).WithMessage("Title must not exceed 500 characters");

        RuleFor(x => x.SortTitle)
            .MaximumLength(500).When(x => !string.IsNullOrEmpty(x.SortTitle))
            .WithMessage("Sort title must not exceed 500 characters");

        RuleFor(x => x.Description)
            .MaximumLength(5000).When(x => !string.IsNullOrEmpty(x.Description))
            .WithMessage("Description must not exceed 5000 characters");

        RuleFor(x => x.ForeignAlbumId)
            .MaximumLength(50).When(x => !string.IsNullOrEmpty(x.ForeignAlbumId))
            .WithMessage("Foreign album ID must not exceed 50 characters");

        RuleFor(x => x.MusicBrainzId)
            .MaximumLength(50).When(x => !string.IsNullOrEmpty(x.MusicBrainzId))
            .WithMessage("MusicBrainz ID must not exceed 50 characters");

        RuleFor(x => x.DiscogsId)
            .MaximumLength(50).When(x => !string.IsNullOrEmpty(x.DiscogsId))
            .WithMessage("Discogs ID must not exceed 50 characters");

        RuleFor(x => x.AlbumType)
            .MaximumLength(50).When(x => !string.IsNullOrEmpty(x.AlbumType))
            .WithMessage("Album type must not exceed 50 characters");

        RuleFor(x => x.Path)
            .MaximumLength(500).When(x => !string.IsNullOrEmpty(x.Path))
            .WithMessage("Path must not exceed 500 characters");

        RuleFor(x => x.RootFolderPath)
            .MaximumLength(500).When(x => !string.IsNullOrEmpty(x.RootFolderPath))
            .WithMessage("Root folder path must not exceed 500 characters");

        RuleFor(x => x.QualityProfileId)
            .GreaterThan(0).WithMessage("Quality profile ID must be greater than 0");

        RuleFor(x => x.ArtistId)
            .GreaterThan(0).When(x => x.ArtistId.HasValue)
            .WithMessage("Artist ID must be greater than 0");

        RuleFor(x => x.Rating)
            .InclusiveBetween(0, 10).When(x => x.Rating.HasValue)
            .WithMessage("Rating must be between 0 and 10");

        RuleFor(x => x.Votes)
            .GreaterThanOrEqualTo(0).When(x => x.Votes.HasValue)
            .WithMessage("Votes must be greater than or equal to 0");

        RuleFor(x => x.TrackCount)
            .GreaterThan(0).When(x => x.TrackCount.HasValue)
            .WithMessage("Track count must be greater than 0");

        RuleFor(x => x.DiscCount)
            .GreaterThan(0).When(x => x.DiscCount.HasValue)
            .WithMessage("Disc count must be greater than 0");

        RuleFor(x => x.Duration)
            .GreaterThan(0).When(x => x.Duration.HasValue)
            .WithMessage("Duration must be greater than 0");
    }
}
