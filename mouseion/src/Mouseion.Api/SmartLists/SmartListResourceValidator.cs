// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation;

namespace Mouseion.Api.SmartLists;

public class SmartListResourceValidator : AbstractValidator<SmartListResource>
{
    public SmartListResourceValidator()
    {
        RuleFor(x => x.Name)
            .NotEmpty().WithMessage("Name is required")
            .MaximumLength(200).WithMessage("Name must not exceed 200 characters");

        RuleFor(x => x.Source)
            .IsInEnum().WithMessage("Invalid source");

        RuleFor(x => x.MediaType)
            .IsInEnum().WithMessage("Invalid media type");

        RuleFor(x => x.RefreshInterval)
            .IsInEnum().WithMessage("Invalid refresh interval");

        RuleFor(x => x.MaxItemsPerRefresh)
            .InclusiveBetween(1, 500).WithMessage("Max items per refresh must be between 1 and 500");

        RuleFor(x => x.MinimumRating)
            .InclusiveBetween(0, 100).When(x => x.MinimumRating.HasValue)
            .WithMessage("Minimum rating must be between 0 and 100");

        RuleFor(x => x.MinYear)
            .InclusiveBetween(1800, 2100).When(x => x.MinYear.HasValue)
            .WithMessage("Min year must be between 1800 and 2100");

        RuleFor(x => x.MaxYear)
            .InclusiveBetween(1800, 2100).When(x => x.MaxYear.HasValue)
            .WithMessage("Max year must be between 1800 and 2100");

        RuleFor(x => x)
            .Must(x => !x.MinYear.HasValue || !x.MaxYear.HasValue || x.MinYear <= x.MaxYear)
            .WithMessage("Min year cannot be greater than max year");

        RuleFor(x => x.RootFolderPath)
            .MaximumLength(1024).WithMessage("Root folder path must not exceed 1024 characters");
    }
}
