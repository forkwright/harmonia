// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation;
using Microsoft.AspNetCore.Mvc;

namespace Mouseion.Api.Validation;

/// <summary>
/// Standard pagination parameters for list endpoints
/// </summary>
public class PaginationRequest
{
    [FromQuery(Name = "page")]
    public int Page { get; set; } = 1;

    [FromQuery(Name = "pageSize")]
    public int PageSize { get; set; } = 50;

    [FromQuery(Name = "sortKey")]
    public string? SortKey { get; set; }

    [FromQuery(Name = "sortDirection")]
    public SortDirection SortDirection { get; set; } = SortDirection.Ascending;
}

public enum SortDirection
{
    Ascending,
    Descending
}

public class PaginationRequestValidator : AbstractValidator<PaginationRequest>
{
    public const int MaxPageSize = 250;
    public const int MinPage = 1;

    public PaginationRequestValidator()
    {
        RuleFor(x => x.Page)
            .GreaterThanOrEqualTo(MinPage)
            .WithMessage($"Page must be at least {MinPage}");

        RuleFor(x => x.PageSize)
            .InclusiveBetween(1, MaxPageSize)
            .WithMessage($"PageSize must be between 1 and {MaxPageSize}");

        RuleFor(x => x.SortKey)
            .MaximumLength(50)
            .When(x => !string.IsNullOrEmpty(x.SortKey))
            .WithMessage("SortKey must not exceed 50 characters");
    }
}
