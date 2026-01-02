// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation;

namespace Mouseion.Api.Tracks;

public class TrackResourceValidator : AbstractValidator<TrackResource>
{
    public TrackResourceValidator()
    {
        RuleFor(x => x.Title)
            .NotEmpty().WithMessage("Title is required")
            .MaximumLength(500).WithMessage("Title must not exceed 500 characters");

        RuleFor(x => x.ForeignTrackId)
            .MaximumLength(50).When(x => !string.IsNullOrEmpty(x.ForeignTrackId))
            .WithMessage("Foreign track ID must not exceed 50 characters");

        RuleFor(x => x.MusicBrainzId)
            .MaximumLength(50).When(x => !string.IsNullOrEmpty(x.MusicBrainzId))
            .WithMessage("MusicBrainz ID must not exceed 50 characters");

        RuleFor(x => x.QualityProfileId)
            .GreaterThan(0).WithMessage("Quality profile ID must be greater than 0");

        RuleFor(x => x.AlbumId)
            .GreaterThan(0).When(x => x.AlbumId.HasValue)
            .WithMessage("Album ID must be greater than 0");

        RuleFor(x => x.ArtistId)
            .GreaterThan(0).When(x => x.ArtistId.HasValue)
            .WithMessage("Artist ID must be greater than 0");

        RuleFor(x => x.TrackNumber)
            .GreaterThan(0).WithMessage("Track number must be greater than 0");

        RuleFor(x => x.DiscNumber)
            .GreaterThan(0).WithMessage("Disc number must be greater than 0");

        RuleFor(x => x.DurationSeconds)
            .GreaterThan(0).When(x => x.DurationSeconds.HasValue)
            .WithMessage("Duration must be greater than 0");
    }
}
