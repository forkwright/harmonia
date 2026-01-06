// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation;

namespace Mouseion.Api.Search;

public class SearchRequestValidator : AbstractValidator<SearchRequest>
{
    public SearchRequestValidator()
    {
        RuleFor(x => x.Q)
            .NotEmpty().WithMessage("Query parameter 'q' is required")
            .MaximumLength(500).WithMessage("Query must not exceed 500 characters");

        RuleFor(x => x.Limit)
            .InclusiveBetween(1, 250).WithMessage("Limit must be between 1 and 250");
    }
}
