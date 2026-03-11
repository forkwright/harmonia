// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation;

namespace Mouseion.Api.Movies;

public class CollectionResourceValidator : AbstractValidator<CollectionResource>
{
    public CollectionResourceValidator()
    {
        RuleFor(x => x.Title)
            .NotEmpty().WithMessage("Title is required")
            .MaximumLength(500).WithMessage("Title must not exceed 500 characters");

        RuleFor(x => x.TmdbId)
            .MaximumLength(50).When(x => !string.IsNullOrEmpty(x.TmdbId))
            .WithMessage("TMDB ID must not exceed 50 characters");

        RuleFor(x => x.QualityProfileId)
            .GreaterThan(0).WithMessage("Quality profile ID must be greater than 0");
    }
}
