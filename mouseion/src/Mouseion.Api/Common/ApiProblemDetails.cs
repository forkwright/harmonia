// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.AspNetCore.Mvc;

namespace Mouseion.Api.Common;

/// <summary>
/// RFC 7807 ProblemDetails response with Mouseion-specific extensions
/// </summary>
public class ApiProblemDetails : ProblemDetails
{
    /// <summary>
    /// Trace ID for request correlation across logs
    /// </summary>
    public string? TraceId { get; set; }

    /// <summary>
    /// Machine-readable error code for client classification
    /// </summary>
    public string? ErrorCode { get; set; }

    /// <summary>
    /// Field-level validation errors (key = field name, value = error messages)
    /// </summary>
    public IDictionary<string, string[]>? Errors { get; set; }
}
