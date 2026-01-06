// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation;

namespace Mouseion.Api.Podcasts;

public class AddPodcastRequestValidator : AbstractValidator<AddPodcastRequest>
{
    public AddPodcastRequestValidator()
    {
        RuleFor(x => x.FeedUrl)
            .NotEmpty().WithMessage("Feed URL is required")
            .Must(BeAValidUrl).WithMessage("Feed URL must be a valid URL");

        RuleFor(x => x.RootFolderPath)
            .MaximumLength(1000).When(x => !string.IsNullOrEmpty(x.RootFolderPath))
            .WithMessage("Root folder path must not exceed 1000 characters");

        RuleFor(x => x.QualityProfileId)
            .GreaterThan(0).WithMessage("Quality profile ID must be greater than 0");
    }

    private bool BeAValidUrl(string url)
    {
        return Uri.TryCreate(url, UriKind.Absolute, out var uriResult)
            && (uriResult.Scheme == Uri.UriSchemeHttp || uriResult.Scheme == Uri.UriSchemeHttps);
    }
}
