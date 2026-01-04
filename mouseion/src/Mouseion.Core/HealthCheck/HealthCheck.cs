// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.RegularExpressions;

namespace Mouseion.Core.HealthCheck;

public partial class HealthCheck
{
    public HealthCheckResult Type { get; set; }
    public string Message { get; set; } = string.Empty;
    public string? WikiUrl { get; set; }

    public HealthCheck(HealthCheckResult type, string message, string? wikiFragment = null)
    {
        Type = type;
        Message = message;

        if (!string.IsNullOrEmpty(wikiFragment))
        {
            WikiUrl = $"https://wiki.servarr.com/mouseion/{wikiFragment}";
        }
        else
        {
            // Generate wiki URL from message
            var fragment = GenerateWikiFragmentRegex().Replace(Message.ToLowerInvariant(), "-");
            WikiUrl = $"https://wiki.servarr.com/mouseion/{fragment}";
        }
    }

    [GeneratedRegex(@"[^a-z0-9]+")]
    private static partial Regex GenerateWikiFragmentRegex();
}
