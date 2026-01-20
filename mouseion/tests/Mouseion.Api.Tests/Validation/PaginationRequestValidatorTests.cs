// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using FluentValidation.TestHelper;
using Mouseion.Api.Validation;
using Xunit;

namespace Mouseion.Api.Tests.Validation;

public class PaginationRequestValidatorTests
{
    private readonly PaginationRequestValidator _validator = new();

    [Fact]
    public void Validate_WithDefaultValues_ShouldPass()
    {
        var request = new PaginationRequest();
        var result = _validator.TestValidate(request);
        result.ShouldNotHaveAnyValidationErrors();
    }

    [Theory]
    [InlineData(1)]
    [InlineData(10)]
    [InlineData(100)]
    public void Validate_WithValidPage_ShouldPass(int page)
    {
        var request = new PaginationRequest { Page = page };
        var result = _validator.TestValidate(request);
        result.ShouldNotHaveValidationErrorFor(x => x.Page);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(-1)]
    [InlineData(-100)]
    public void Validate_WithInvalidPage_ShouldFail(int page)
    {
        var request = new PaginationRequest { Page = page };
        var result = _validator.TestValidate(request);
        result.ShouldHaveValidationErrorFor(x => x.Page);
    }

    [Theory]
    [InlineData(1)]
    [InlineData(50)]
    [InlineData(250)]
    public void Validate_WithValidPageSize_ShouldPass(int pageSize)
    {
        var request = new PaginationRequest { PageSize = pageSize };
        var result = _validator.TestValidate(request);
        result.ShouldNotHaveValidationErrorFor(x => x.PageSize);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(-1)]
    [InlineData(251)]
    [InlineData(1000)]
    public void Validate_WithInvalidPageSize_ShouldFail(int pageSize)
    {
        var request = new PaginationRequest { PageSize = pageSize };
        var result = _validator.TestValidate(request);
        result.ShouldHaveValidationErrorFor(x => x.PageSize);
    }

    [Fact]
    public void Validate_WithNullSortKey_ShouldPass()
    {
        var request = new PaginationRequest { SortKey = null };
        var result = _validator.TestValidate(request);
        result.ShouldNotHaveValidationErrorFor(x => x.SortKey);
    }

    [Fact]
    public void Validate_WithValidSortKey_ShouldPass()
    {
        var request = new PaginationRequest { SortKey = "title" };
        var result = _validator.TestValidate(request);
        result.ShouldNotHaveValidationErrorFor(x => x.SortKey);
    }

    [Fact]
    public void Validate_WithTooLongSortKey_ShouldFail()
    {
        var request = new PaginationRequest { SortKey = new string('a', 51) };
        var result = _validator.TestValidate(request);
        result.ShouldHaveValidationErrorFor(x => x.SortKey);
    }

    [Fact]
    public void MaxPageSize_ShouldBe250()
    {
        Assert.Equal(250, PaginationRequestValidator.MaxPageSize);
    }

    [Fact]
    public void MinPage_ShouldBe1()
    {
        Assert.Equal(1, PaginationRequestValidator.MinPage);
    }
}
