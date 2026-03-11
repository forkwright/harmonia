// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation;

namespace Mouseion.Api.Artists;

public class ArtistResourceValidator : AbstractValidator<ArtistResource>
{
    public ArtistResourceValidator()
    {
        RuleFor(x => x.Name)
            .NotEmpty().WithMessage("Name is required")
            .MaximumLength(200).WithMessage("Name must not exceed 200 characters");

        RuleFor(x => x.SortName)
            .MaximumLength(200).When(x => !string.IsNullOrEmpty(x.SortName))
            .WithMessage("Sort name must not exceed 200 characters");

        RuleFor(x => x.Description)
            .MaximumLength(5000).When(x => !string.IsNullOrEmpty(x.Description))
            .WithMessage("Description must not exceed 5000 characters");

        RuleFor(x => x.MusicBrainzId)
            .MaximumLength(50).When(x => !string.IsNullOrEmpty(x.MusicBrainzId))
            .WithMessage("MusicBrainz ID must not exceed 50 characters");

        RuleFor(x => x.SpotifyId)
            .MaximumLength(50).When(x => !string.IsNullOrEmpty(x.SpotifyId))
            .WithMessage("Spotify ID must not exceed 50 characters");

        RuleFor(x => x.LastFmId)
            .MaximumLength(50).When(x => !string.IsNullOrEmpty(x.LastFmId))
            .WithMessage("Last.fm ID must not exceed 50 characters");

        RuleFor(x => x.Country)
            .MaximumLength(100).When(x => !string.IsNullOrEmpty(x.Country))
            .WithMessage("Country must not exceed 100 characters");

        RuleFor(x => x.Path)
            .MaximumLength(500).When(x => !string.IsNullOrEmpty(x.Path))
            .WithMessage("Path must not exceed 500 characters");

        RuleFor(x => x.RootFolderPath)
            .MaximumLength(500).When(x => !string.IsNullOrEmpty(x.RootFolderPath))
            .WithMessage("Root folder path must not exceed 500 characters");

        RuleFor(x => x.QualityProfileId)
            .GreaterThan(0).WithMessage("Quality profile ID must be greater than 0");

        RuleFor(x => x.Rating)
            .InclusiveBetween(0, 10).When(x => x.Rating.HasValue)
            .WithMessage("Rating must be between 0 and 10");

        RuleFor(x => x.Votes)
            .GreaterThanOrEqualTo(0).When(x => x.Votes.HasValue)
            .WithMessage("Votes must be greater than or equal to 0");
    }
}
