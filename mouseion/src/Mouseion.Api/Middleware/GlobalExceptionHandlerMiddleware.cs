// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Net;
using System.Text.Json;
using System.Text.Json.Serialization;
using FluentValidation;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Logging;
using Mouseion.Api.Common;
using Mouseion.Common.Disk;
using Mouseion.Common.Exceptions;
using Mouseion.Common.Extensions;
using Mouseion.Common.Http;
using Mouseion.Core.ImportLists.Exceptions;

namespace Mouseion.Api.Middleware;

/// <summary>
/// Global exception handler that converts exceptions to RFC 7807 ProblemDetails responses
/// </summary>
public class GlobalExceptionHandlerMiddleware
{
    private readonly RequestDelegate _next;
    private readonly ILogger<GlobalExceptionHandlerMiddleware> _logger;
    private readonly JsonSerializerOptions _jsonOptions;

    public GlobalExceptionHandlerMiddleware(
        RequestDelegate next,
        ILogger<GlobalExceptionHandlerMiddleware> logger)
    {
        _next = next;
        _logger = logger;
        _jsonOptions = new JsonSerializerOptions
        {
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
        };
    }

    public async Task InvokeAsync(HttpContext context)
    {
        try
        {
            await _next(context);
        }
        catch (Exception ex) // lgtm[cs/catch-of-all-exceptions] - Global exception handler needs to catch everything
        {
            await HandleExceptionAsync(context, ex);
        }
    }

    private async Task HandleExceptionAsync(HttpContext context, Exception exception)
    {
        var (statusCode, errorCode) = MapExceptionToStatusCode(exception);
        var traceId = context.TraceIdentifier;

        var problemDetails = new ApiProblemDetails
        {
            Status = (int)statusCode,
            Title = GetTitle(statusCode),
            Detail = GetSafeMessage(exception, statusCode),
            Instance = context.Request.Path,
            TraceId = traceId,
            ErrorCode = errorCode
        };

        if (exception is ValidationException validationException)
        {
            problemDetails.Errors = validationException.Errors
                .GroupBy(e => e.PropertyName)
                .ToDictionary(
                    g => g.Key,
                    g => g.Select(e => e.ErrorMessage).ToArray()
                );
        }

        LogException(context, exception, statusCode, traceId);

        context.Response.ContentType = "application/problem+json";
        context.Response.StatusCode = (int)statusCode;

        var json = JsonSerializer.Serialize(problemDetails, _jsonOptions);
        await context.Response.WriteAsync(json);
    }

    private (HttpStatusCode statusCode, string errorCode) MapExceptionToStatusCode(Exception exception)
    {
        return exception switch
        {
            // FluentValidation
            ValidationException => (HttpStatusCode.UnprocessableEntity, "VALIDATION_ERROR"),

            // Mouseion-specific exceptions
            TooManyRequestsException => (HttpStatusCode.TooManyRequests, "RATE_LIMITED"),
            InvalidHeaderException => (HttpStatusCode.BadRequest, "INVALID_HEADER"),
            InvalidRequestException => (HttpStatusCode.BadRequest, "INVALID_REQUEST"),
            TlsFailureException => (HttpStatusCode.BadGateway, "TLS_FAILURE"),
            UnexpectedHtmlContentException => (HttpStatusCode.BadGateway, "UNEXPECTED_RESPONSE"),
            FileAlreadyExistsException => (HttpStatusCode.Conflict, "FILE_EXISTS"),
            PathCombinationException => (HttpStatusCode.BadRequest, "INVALID_PATH"),
            DestinationAlreadyExistsException => (HttpStatusCode.Conflict, "DESTINATION_EXISTS"),
            ImportListException => (HttpStatusCode.BadRequest, "IMPORT_LIST_ERROR"),

            // .NET built-in exceptions (specific before general)
            ArgumentNullException => (HttpStatusCode.BadRequest, "BAD_REQUEST"),
            ArgumentException => (HttpStatusCode.BadRequest, "BAD_REQUEST"),
            FileNotFoundException => (HttpStatusCode.NotFound, "FILE_NOT_FOUND"),
            DirectoryNotFoundException => (HttpStatusCode.NotFound, "DIRECTORY_NOT_FOUND"),
            UnauthorizedAccessException => (HttpStatusCode.Forbidden, "FORBIDDEN"),
            InvalidOperationException => (HttpStatusCode.Conflict, "CONFLICT"),
            TaskCanceledException => ((HttpStatusCode)499, "CLIENT_CLOSED_REQUEST"),
            OperationCanceledException => ((HttpStatusCode)499, "REQUEST_CANCELLED"),
            HttpRequestException => (HttpStatusCode.BadGateway, "EXTERNAL_SERVICE_ERROR"),

            _ => (HttpStatusCode.InternalServerError, "INTERNAL_ERROR")
        };
    }

    private static string GetTitle(HttpStatusCode statusCode)
    {
        return statusCode switch
        {
            HttpStatusCode.BadRequest => "Bad Request",
            HttpStatusCode.Forbidden => "Forbidden",
            HttpStatusCode.NotFound => "Not Found",
            HttpStatusCode.Conflict => "Conflict",
            HttpStatusCode.UnprocessableEntity => "Validation Failed",
            HttpStatusCode.TooManyRequests => "Too Many Requests",
            HttpStatusCode.BadGateway => "External Service Error",
            HttpStatusCode.InternalServerError => "Internal Server Error",
            _ => "Error"
        };
    }

    private static string GetSafeMessage(Exception exception, HttpStatusCode statusCode)
    {
        // For 500 errors, don't leak internal details to clients
        if (statusCode == HttpStatusCode.InternalServerError)
        {
            return "An unexpected error occurred. Please contact support with the trace ID.";
        }

        // For validation errors, the Errors dictionary will contain details
        if (exception is ValidationException)
        {
            return "One or more validation errors occurred.";
        }

        // For other errors, return the exception message (safe for client consumption)
        return exception.Message;
    }

    private void LogException(HttpContext context, Exception exception, HttpStatusCode statusCode, string traceId)
    {
        var logLevel = statusCode switch
        {
            HttpStatusCode.InternalServerError => LogLevel.Error,
            HttpStatusCode.BadGateway => LogLevel.Error,
            _ => LogLevel.Warning
        };

        var logMessage = "HTTP {StatusCode} - {Method} {Path} - TraceId: {TraceId}";
        var method = context.Request.Method.SanitizeForLog();
        var path = context.Request.Path.ToString().SanitizeForLog();

        if (logLevel == LogLevel.Error)
        {
            _logger.Log(logLevel, exception, logMessage, (int)statusCode, method, path, traceId);
        }
        else
        {
            _logger.Log(logLevel, logMessage + " - {Message}", (int)statusCode, method, path, traceId, exception.Message.SanitizeForLog());
        }
    }
}
