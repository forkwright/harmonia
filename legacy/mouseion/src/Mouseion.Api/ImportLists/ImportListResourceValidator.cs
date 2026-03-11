// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using FluentValidation;

namespace Mouseion.Api.ImportLists;

public class ImportListResourceValidator : AbstractValidator<ImportListResource>
{
    public ImportListResourceValidator()
    {
        RuleFor(x => x.Name)
            .NotEmpty().WithMessage("Name is required")
            .MaximumLength(200).WithMessage("Name must not exceed 200 characters");

        RuleFor(x => x.Implementation)
            .NotEmpty().WithMessage("Implementation is required")
            .MaximumLength(100).WithMessage("Implementation must not exceed 100 characters");

        RuleFor(x => x.QualityProfileId)
            .GreaterThan(0).WithMessage("Quality profile ID must be greater than 0");

        RuleFor(x => x.RootFolderPath)
            .NotEmpty().WithMessage("Root folder path is required")
            .MaximumLength(1000).WithMessage("Root folder path must not exceed 1000 characters");

        RuleFor(x => x.MinRefreshInterval)
            .Must(interval => interval >= TimeSpan.FromMinutes(5))
            .WithMessage("Minimum refresh interval must be at least 5 minutes");

        RuleFor(x => x.Settings)
            .NotEmpty().WithMessage("Settings are required")
            .Must(BeValidJson).WithMessage("Settings must be valid JSON");
    }

    private bool BeValidJson(string json)
    {
        if (string.IsNullOrWhiteSpace(json))
            return false;

        try
        {
            JsonDocument.Parse(json);
            return true;
        }
        catch (JsonException)
        {
            return false;
        }
    }
}
