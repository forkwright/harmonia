// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation;

namespace Mouseion.Api.TV;

public class SeriesResourceValidator : AbstractValidator<SeriesResource>
{
    public SeriesResourceValidator()
    {
        RuleFor(x => x.Title)
            .NotEmpty().WithMessage("Title is required")
            .MaximumLength(500).WithMessage("Title must not exceed 500 characters");

        RuleFor(x => x.Year)
            .InclusiveBetween(1900, 2100).WithMessage("Year must be between 1900 and 2100");

        RuleFor(x => x.TvdbId)
            .NotEmpty().When(x => x.Id == 0 && !x.TmdbId.HasValue && string.IsNullOrEmpty(x.ImdbId))
            .WithMessage("At least one of TvdbId, TmdbId, or ImdbId is required for new series");

        RuleFor(x => x.ImdbId)
            .MaximumLength(20).When(x => !string.IsNullOrEmpty(x.ImdbId))
            .WithMessage("IMDB ID must not exceed 20 characters");

        RuleFor(x => x.Path)
            .NotEmpty().WithMessage("Path is required")
            .MaximumLength(1000).WithMessage("Path must not exceed 1000 characters");

        RuleFor(x => x.QualityProfileId)
            .GreaterThan(0).WithMessage("Quality profile ID must be greater than 0");

        RuleFor(x => x.Runtime)
            .GreaterThan(0).When(x => x.Runtime.HasValue)
            .WithMessage("Runtime must be greater than 0");

        RuleFor(x => x.Status)
            .Must(status => status == "Continuing" || status == "Ended")
            .WithMessage("Status must be either 'Continuing' or 'Ended'");
    }
}
