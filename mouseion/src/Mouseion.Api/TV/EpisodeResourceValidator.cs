// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation;

namespace Mouseion.Api.TV;

public class EpisodeResourceValidator : AbstractValidator<EpisodeResource>
{
    public EpisodeResourceValidator()
    {
        RuleFor(x => x.SeriesId)
            .GreaterThan(0).WithMessage("Series ID must be greater than 0");

        RuleFor(x => x.SeasonNumber)
            .GreaterThanOrEqualTo(0).WithMessage("Season number must be 0 or greater");

        RuleFor(x => x.EpisodeNumber)
            .GreaterThan(0).WithMessage("Episode number must be greater than 0");

        RuleFor(x => x.Title)
            .MaximumLength(500).When(x => !string.IsNullOrEmpty(x.Title))
            .WithMessage("Title must not exceed 500 characters");

        RuleFor(x => x.AbsoluteEpisodeNumber)
            .GreaterThan(0).When(x => x.AbsoluteEpisodeNumber.HasValue)
            .WithMessage("Absolute episode number must be greater than 0");

        RuleFor(x => x.SceneSeasonNumber)
            .GreaterThanOrEqualTo(0).When(x => x.SceneSeasonNumber.HasValue)
            .WithMessage("Scene season number must be 0 or greater");

        RuleFor(x => x.SceneEpisodeNumber)
            .GreaterThan(0).When(x => x.SceneEpisodeNumber.HasValue)
            .WithMessage("Scene episode number must be greater than 0");

        RuleFor(x => x.EpisodeFileId)
            .GreaterThan(0).When(x => x.EpisodeFileId.HasValue)
            .WithMessage("Episode file ID must be greater than 0");
    }
}
