// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation;

namespace Mouseion.Api.DelayProfiles;

public class DelayProfileResourceValidator : AbstractValidator<DelayProfileResource>
{
    public DelayProfileResourceValidator()
    {
        RuleFor(x => x.Name)
            .NotEmpty().WithMessage("Name is required")
            .MaximumLength(200).WithMessage("Name must not exceed 200 characters");

        RuleFor(x => x.UsenetDelay)
            .GreaterThanOrEqualTo(0).WithMessage("Usenet delay cannot be negative")
            .LessThanOrEqualTo(720).WithMessage("Usenet delay cannot exceed 30 days (720 hours)");

        RuleFor(x => x.TorrentDelay)
            .GreaterThanOrEqualTo(0).WithMessage("Torrent delay cannot be negative")
            .LessThanOrEqualTo(720).WithMessage("Torrent delay cannot exceed 30 days (720 hours)");

        RuleFor(x => x.PreferredQualityWeight)
            .GreaterThanOrEqualTo(0).WithMessage("Preferred quality weight cannot be negative");

        RuleFor(x => x.Order)
            .GreaterThanOrEqualTo(0).WithMessage("Order cannot be negative");

        RuleFor(x => x.PreferredProtocol)
            .IsInEnum().WithMessage("Invalid preferred protocol");

        RuleFor(x => x.MediaType)
            .IsInEnum().When(x => x.MediaType.HasValue)
            .WithMessage("Invalid media type");
    }
}
