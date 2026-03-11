// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Text.Json;
using FluentAssertions;
using FluentValidation;
using FluentValidation.Results;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Logging.Abstractions;
using Mouseion.Api.Common;
using Mouseion.Api.Middleware;
using Mouseion.Common.Disk;
using Mouseion.Common.Exceptions;
using Mouseion.Common.Http;
using Mouseion.Core.ImportLists.Exceptions;
using Xunit;

namespace Mouseion.Api.Tests.Middleware;

public class GlobalExceptionHandlerMiddlewareTests
{
    [Fact]
    public async Task InvokeAsync_NoException_CallsNextMiddleware()
    {
        // Arrange
        var context = new DefaultHttpContext();
        var nextCalled = false;
        RequestDelegate next = (ctx) =>
        {
            nextCalled = true;
            return Task.CompletedTask;
        };

        var middleware = new GlobalExceptionHandlerMiddleware(next, NullLogger<GlobalExceptionHandlerMiddleware>.Instance);

        // Act
        await middleware.InvokeAsync(context);

        // Assert
        nextCalled.Should().BeTrue();
        context.Response.StatusCode.Should().Be(200);
    }

    [Fact]
    public async Task InvokeAsync_ValidationException_Returns422WithProblemDetails()
    {
        // Arrange
        var context = new DefaultHttpContext();
        context.Response.Body = new MemoryStream();
        context.Request.Path = "/api/v3/movies";

        var validationFailures = new List<ValidationFailure>
        {
            new ValidationFailure("Title", "Title is required"),
            new ValidationFailure("Year", "Year must be between 1900 and 2100")
        };
        var validationException = new ValidationException(validationFailures);

        RequestDelegate next = (ctx) => throw validationException;

        var middleware = new GlobalExceptionHandlerMiddleware(next, NullLogger<GlobalExceptionHandlerMiddleware>.Instance);

        // Act
        await middleware.InvokeAsync(context);

        // Assert
        context.Response.StatusCode.Should().Be((int)HttpStatusCode.UnprocessableEntity);
        context.Response.ContentType.Should().Be("application/problem+json");

        context.Response.Body.Seek(0, SeekOrigin.Begin);
        var responseBody = await new StreamReader(context.Response.Body).ReadToEndAsync();
        var problemDetails = JsonSerializer.Deserialize<ApiProblemDetails>(responseBody, new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true
        });

        problemDetails.Should().NotBeNull();
        problemDetails!.Status.Should().Be(422);
        problemDetails.Title.Should().Be("Validation Failed");
        problemDetails.ErrorCode.Should().Be("VALIDATION_ERROR");
        problemDetails.Errors.Should().ContainKey("Title");
        problemDetails.Errors.Should().ContainKey("Year");
    }

    [Theory]
    [InlineData(typeof(InvalidHeaderException), HttpStatusCode.BadRequest, "INVALID_HEADER")]
    [InlineData(typeof(InvalidRequestException), HttpStatusCode.BadRequest, "INVALID_REQUEST")]
    [InlineData(typeof(PathCombinationException), HttpStatusCode.BadRequest, "INVALID_PATH")]
    [InlineData(typeof(DestinationAlreadyExistsException), HttpStatusCode.Conflict, "DESTINATION_EXISTS")]
    [InlineData(typeof(ImportListException), HttpStatusCode.BadRequest, "IMPORT_LIST_ERROR")]
    public async Task InvokeAsync_MouseionException_ReturnsMappedStatusCodeAndErrorCode(
        Type exceptionType, HttpStatusCode expectedStatusCode, string expectedErrorCode)
    {
        // Arrange
        var context = new DefaultHttpContext();
        context.Response.Body = new MemoryStream();

        var exception = (Exception)Activator.CreateInstance(exceptionType, "Test error")!;
        RequestDelegate next = (ctx) => throw exception;

        var middleware = new GlobalExceptionHandlerMiddleware(next, NullLogger<GlobalExceptionHandlerMiddleware>.Instance);

        // Act
        await middleware.InvokeAsync(context);

        // Assert
        context.Response.StatusCode.Should().Be((int)expectedStatusCode);

        context.Response.Body.Seek(0, SeekOrigin.Begin);
        var responseBody = await new StreamReader(context.Response.Body).ReadToEndAsync();
        var problemDetails = JsonSerializer.Deserialize<ApiProblemDetails>(responseBody, new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true
        });

        problemDetails!.ErrorCode.Should().Be(expectedErrorCode);
    }

    [Theory]
    [InlineData(typeof(ArgumentNullException), HttpStatusCode.BadRequest, "BAD_REQUEST")]
    [InlineData(typeof(ArgumentException), HttpStatusCode.BadRequest, "BAD_REQUEST")]
    [InlineData(typeof(FileNotFoundException), HttpStatusCode.NotFound, "FILE_NOT_FOUND")]
    [InlineData(typeof(DirectoryNotFoundException), HttpStatusCode.NotFound, "DIRECTORY_NOT_FOUND")]
    [InlineData(typeof(UnauthorizedAccessException), HttpStatusCode.Forbidden, "FORBIDDEN")]
    [InlineData(typeof(InvalidOperationException), HttpStatusCode.Conflict, "CONFLICT")]
    [InlineData(typeof(HttpRequestException), HttpStatusCode.BadGateway, "EXTERNAL_SERVICE_ERROR")]
    public async Task InvokeAsync_BuiltInException_ReturnsMappedStatusCodeAndErrorCode(
        Type exceptionType, HttpStatusCode expectedStatusCode, string expectedErrorCode)
    {
        // Arrange
        var context = new DefaultHttpContext();
        context.Response.Body = new MemoryStream();

        var exception = (Exception)Activator.CreateInstance(exceptionType, "Test error")!;
        RequestDelegate next = (ctx) => throw exception;

        var middleware = new GlobalExceptionHandlerMiddleware(next, NullLogger<GlobalExceptionHandlerMiddleware>.Instance);

        // Act
        await middleware.InvokeAsync(context);

        // Assert
        context.Response.StatusCode.Should().Be((int)expectedStatusCode);

        context.Response.Body.Seek(0, SeekOrigin.Begin);
        var responseBody = await new StreamReader(context.Response.Body).ReadToEndAsync();
        var problemDetails = JsonSerializer.Deserialize<ApiProblemDetails>(responseBody, new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true
        });

        problemDetails!.ErrorCode.Should().Be(expectedErrorCode);
    }

    [Fact]
    public async Task InvokeAsync_UnknownException_Returns500WithGenericMessage()
    {
        // Arrange
        var context = new DefaultHttpContext();
        context.Response.Body = new MemoryStream();

        RequestDelegate next = (ctx) => throw new Exception("Internal error details");

        var middleware = new GlobalExceptionHandlerMiddleware(next, NullLogger<GlobalExceptionHandlerMiddleware>.Instance);

        // Act
        await middleware.InvokeAsync(context);

        // Assert
        context.Response.StatusCode.Should().Be((int)HttpStatusCode.InternalServerError);

        context.Response.Body.Seek(0, SeekOrigin.Begin);
        var responseBody = await new StreamReader(context.Response.Body).ReadToEndAsync();
        var problemDetails = JsonSerializer.Deserialize<ApiProblemDetails>(responseBody, new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true
        });

        problemDetails!.Status.Should().Be(500);
        problemDetails.Title.Should().Be("Internal Server Error");
        problemDetails.ErrorCode.Should().Be("INTERNAL_ERROR");
        problemDetails.Detail.Should().Be("An unexpected error occurred. Please contact support with the trace ID.");
        problemDetails.Detail.Should().NotContain("Internal error details"); // Should not leak internal details
    }

    [Fact]
    public async Task InvokeAsync_IncludesTraceId_InProblemDetails()
    {
        // Arrange
        var context = new DefaultHttpContext();
        context.Response.Body = new MemoryStream();
        context.TraceIdentifier = "test-trace-123";

        RequestDelegate next = (ctx) => throw new ArgumentException("Test error");

        var middleware = new GlobalExceptionHandlerMiddleware(next, NullLogger<GlobalExceptionHandlerMiddleware>.Instance);

        // Act
        await middleware.InvokeAsync(context);

        // Assert
        context.Response.Body.Seek(0, SeekOrigin.Begin);
        var responseBody = await new StreamReader(context.Response.Body).ReadToEndAsync();
        var problemDetails = JsonSerializer.Deserialize<ApiProblemDetails>(responseBody, new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true
        });

        problemDetails!.TraceId.Should().Be("test-trace-123");
    }

    [Fact]
    public async Task InvokeAsync_IncludesRequestPath_InProblemDetails()
    {
        // Arrange
        var context = new DefaultHttpContext();
        context.Response.Body = new MemoryStream();
        context.Request.Path = "/api/v3/movies/123";

        RequestDelegate next = (ctx) => throw new ArgumentException("Test error");

        var middleware = new GlobalExceptionHandlerMiddleware(next, NullLogger<GlobalExceptionHandlerMiddleware>.Instance);

        // Act
        await middleware.InvokeAsync(context);

        // Assert
        context.Response.Body.Seek(0, SeekOrigin.Begin);
        var responseBody = await new StreamReader(context.Response.Body).ReadToEndAsync();
        var problemDetails = JsonSerializer.Deserialize<ApiProblemDetails>(responseBody, new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true
        });

        problemDetails!.Instance.Should().Be("/api/v3/movies/123");
    }

    [Theory]
    [InlineData(typeof(TaskCanceledException), 499, "CLIENT_CLOSED_REQUEST")]
    [InlineData(typeof(OperationCanceledException), 499, "REQUEST_CANCELLED")]
    public async Task InvokeAsync_CancellationException_Returns499(
        Type exceptionType, int expectedStatusCode, string expectedErrorCode)
    {
        // Arrange
        var context = new DefaultHttpContext();
        context.Response.Body = new MemoryStream();

        var exception = (Exception)Activator.CreateInstance(exceptionType)!;
        RequestDelegate next = (ctx) => throw exception;

        var middleware = new GlobalExceptionHandlerMiddleware(next, NullLogger<GlobalExceptionHandlerMiddleware>.Instance);

        // Act
        await middleware.InvokeAsync(context);

        // Assert
        context.Response.StatusCode.Should().Be(expectedStatusCode);

        context.Response.Body.Seek(0, SeekOrigin.Begin);
        var responseBody = await new StreamReader(context.Response.Body).ReadToEndAsync();
        var problemDetails = JsonSerializer.Deserialize<ApiProblemDetails>(responseBody, new JsonSerializerOptions
        {
            PropertyNameCaseInsensitive = true
        });

        problemDetails!.ErrorCode.Should().Be(expectedErrorCode);
    }
}
