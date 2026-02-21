// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using FluentValidation;

namespace Mouseion.Api.SmartPlaylists;

public class SmartPlaylistResourceValidator : AbstractValidator<SmartPlaylistResource>
{
    public SmartPlaylistResourceValidator()
    {
        RuleFor(x => x.Name)
            .NotEmpty().WithMessage("Name is required")
            .MaximumLength(200).WithMessage("Name must not exceed 200 characters");

        RuleFor(x => x.FilterRequestJson)
            .NotEmpty().WithMessage("FilterRequestJson is required")
            .Must(BeValidJson).WithMessage("FilterRequestJson must be valid JSON");
    }

    private static bool BeValidJson(string json)
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
