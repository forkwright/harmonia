// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Api.Common;

namespace Mouseion.Api.Tests.Common;

public class ApiProblemDetailsTests
{
    [Fact]
    public void Constructor_CreatesInstance_WithDefaultValues()
    {
        var problemDetails = new ApiProblemDetails();

        Assert.Null(problemDetails.TraceId);
        Assert.Null(problemDetails.ErrorCode);
        Assert.Null(problemDetails.Errors);
        Assert.Null(problemDetails.Title);
        Assert.Null(problemDetails.Detail);
        Assert.Null(problemDetails.Instance);
        Assert.Null(problemDetails.Status);
    }

    [Fact]
    public void TraceId_CanBeSet_AndRetrieved()
    {
        var problemDetails = new ApiProblemDetails { TraceId = "trace-123" };
        Assert.Equal("trace-123", problemDetails.TraceId);
    }

    [Fact]
    public void ErrorCode_CanBeSet_AndRetrieved()
    {
        var problemDetails = new ApiProblemDetails { ErrorCode = "TEST_ERROR" };
        Assert.Equal("TEST_ERROR", problemDetails.ErrorCode);
    }

    [Fact]
    public void Errors_CanBeSet_AndRetrieved()
    {
        var errors = new Dictionary<string, string[]>
        {
            { "field1", new[] { "error1", "error2" } }
        };
        var problemDetails = new ApiProblemDetails { Errors = errors };

        Assert.NotNull(problemDetails.Errors);
        Assert.Single(problemDetails.Errors);
        Assert.Equal(2, problemDetails.Errors["field1"].Length);
    }

    [Fact]
    public void Properties_CanAllBeSet_Together()
    {
        var problemDetails = new ApiProblemDetails
        {
            TraceId = "trace-123",
            ErrorCode = "TEST_ERROR",
            Errors = new Dictionary<string, string[]>
            {
                { "field1", new[] { "error1", "error2" } },
                { "field2", new[] { "error3" } }
            },
            Title = "Test Title",
            Detail = "Test Detail",
            Instance = "/api/test",
            Status = 400
        };

        Assert.Equal("trace-123", problemDetails.TraceId);
        Assert.Equal("TEST_ERROR", problemDetails.ErrorCode);
        Assert.NotNull(problemDetails.Errors);
        Assert.Equal(2, problemDetails.Errors.Count);
        Assert.Equal("Test Title", problemDetails.Title);
        Assert.Equal("Test Detail", problemDetails.Detail);
        Assert.Equal("/api/test", problemDetails.Instance);
        Assert.Equal(400, problemDetails.Status);
    }

    [Fact]
    public void Errors_CanContainMultipleFields_WithMultipleMessages()
    {
        var problemDetails = new ApiProblemDetails
        {
            Errors = new Dictionary<string, string[]>
            {
                { "email", new[] { "Email is required", "Email must be valid" } },
                { "password", new[] { "Password is too short" } },
                { "username", new[] { "Username is taken" } }
            }
        };

        Assert.Equal(3, problemDetails.Errors.Count);
        Assert.Equal(2, problemDetails.Errors["email"].Length);
        Assert.Single(problemDetails.Errors["password"]);
        Assert.Single(problemDetails.Errors["username"]);
    }

    [Fact]
    public void InheritsFromProblemDetails_AndCanBeUsedAsBase()
    {
        var problemDetails = new ApiProblemDetails
        {
            Type = "https://tools.ietf.org/html/rfc7231#section-6.5.1",
            Title = "Bad Request",
            Status = 400,
            Detail = "Invalid request parameters",
            Instance = "/api/v3/movies"
        };

        Microsoft.AspNetCore.Mvc.ProblemDetails baseDetails = problemDetails;

        Assert.Equal("https://tools.ietf.org/html/rfc7231#section-6.5.1", baseDetails.Type);
        Assert.Equal("Bad Request", baseDetails.Title);
        Assert.Equal(400, baseDetails.Status);
        Assert.Equal("Invalid request parameters", baseDetails.Detail);
        Assert.Equal("/api/v3/movies", baseDetails.Instance);
    }

    [Fact]
    public void Errors_CanBeEmpty()
    {
        var problemDetails = new ApiProblemDetails
        {
            Errors = new Dictionary<string, string[]>()
        };

        Assert.NotNull(problemDetails.Errors);
        Assert.Empty(problemDetails.Errors);
    }

    [Fact]
    public void Extensions_DictionaryWorks_WithCustomProperties()
    {
        var problemDetails = new ApiProblemDetails
        {
            Title = "Error"
        };

        problemDetails.Extensions["custom"] = "value";
        problemDetails.Extensions["count"] = 42;

        Assert.Equal(2, problemDetails.Extensions.Count);
        Assert.Equal("value", problemDetails.Extensions["custom"]);
        Assert.Equal(42, problemDetails.Extensions["count"]);
    }
}
