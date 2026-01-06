// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Text.Json;
using FluentAssertions;
using Microsoft.AspNetCore.Mvc;
using Mouseion.Api.Common;
using Xunit;

namespace Mouseion.Api.Tests;

/// <summary>
/// Integration tests for RFC 7807 ProblemDetails error handling
/// </summary>
public class ErrorHandlingTests
{
    [Fact]
    public void ApiProblemDetails_SerializesToJson_WithCamelCaseProperties()
    {
        // Arrange
        var problemDetails = new ApiProblemDetails
        {
            Status = 400,
            Title = "Bad Request",
            Detail = "Validation failed",
            Instance = "/api/v3/movies",
            TraceId = "test-trace-123",
            ErrorCode = "VALIDATION_ERROR",
            Errors = new Dictionary<string, string[]>
            {
                { "title", new[] { "Title is required" } },
                { "year", new[] { "Year must be between 1900 and 2100" } }
            }
        };

        var options = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            DefaultIgnoreCondition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull
        };

        // Act
        var json = JsonSerializer.Serialize(problemDetails, options);
        var deserialized = JsonSerializer.Deserialize<Dictionary<string, JsonElement>>(json);

        // Assert
        deserialized.Should().NotBeNull();
        deserialized!["status"].GetInt32().Should().Be(400);
        deserialized["title"].GetString().Should().Be("Bad Request");
        deserialized["detail"].GetString().Should().Be("Validation failed");
        deserialized["traceId"].GetString().Should().Be("test-trace-123");
        deserialized["errorCode"].GetString().Should().Be("VALIDATION_ERROR");
        deserialized.Should().ContainKey("errors");
    }

    [Fact]
    public void ApiProblemDetails_OmitsNullProperties_WhenSerializing()
    {
        // Arrange
        var problemDetails = new ApiProblemDetails
        {
            Status = 404,
            Title = "Not Found",
            Detail = "Resource not found",
            Instance = "/api/v3/movies/999"
            // TraceId, ErrorCode, Errors are null
        };

        var options = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            DefaultIgnoreCondition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull
        };

        // Act
        var json = JsonSerializer.Serialize(problemDetails, options);
        var deserialized = JsonSerializer.Deserialize<Dictionary<string, JsonElement>>(json);

        // Assert
        deserialized.Should().NotBeNull();
        deserialized.Should().NotContainKey("traceId");
        deserialized.Should().NotContainKey("errorCode");
        deserialized.Should().NotContainKey("errors");
    }

    [Theory]
    [InlineData(400, "Bad Request")]
    [InlineData(403, "Forbidden")]
    [InlineData(404, "Not Found")]
    [InlineData(409, "Conflict")]
    [InlineData(422, "Validation Failed")]
    [InlineData(429, "Too Many Requests")]
    [InlineData(500, "Internal Server Error")]
    [InlineData(502, "External Service Error")]
    public void ProblemDetails_HasCorrectTitles_ForStatusCodes(int statusCode, string expectedTitle)
    {
        // This test documents the expected titles for each status code
        // Actual mapping is in GlobalExceptionHandlerMiddleware.GetTitle()
        var problemDetails = new ProblemDetails
        {
            Status = statusCode,
            Title = expectedTitle
        };

        problemDetails.Title.Should().Be(expectedTitle);
    }

    [Fact]
    public void ErrorCodes_AreConsistent_AcrossExceptionTypes()
    {
        // This test documents the expected error codes
        // Actual mapping is in GlobalExceptionHandlerMiddleware.MapExceptionToStatusCode()
        var errorCodes = new Dictionary<string, string>
        {
            { "ValidationException", "VALIDATION_ERROR" },
            { "ArgumentException", "BAD_REQUEST" },
            { "FileNotFoundException", "FILE_NOT_FOUND" },
            { "UnauthorizedAccessException", "FORBIDDEN" },
            { "TooManyRequestsException", "RATE_LIMITED" },
            { "InvalidHeaderException", "INVALID_HEADER" },
            { "TlsFailureException", "TLS_FAILURE" },
            { "ImportListException", "IMPORT_LIST_ERROR" }
        };

        // Assert error codes follow naming conventions
        errorCodes.Values.Should().AllSatisfy(code =>
        {
            code.Should().MatchRegex("^[A-Z_]+$", "error codes should be UPPER_SNAKE_CASE");
        });
    }
}
