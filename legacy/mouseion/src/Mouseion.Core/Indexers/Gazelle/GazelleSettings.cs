// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Indexers.Gazelle;

public class GazelleSettings : IndexerSettings
{
    public GazelleSettings()
    {
        BaseUrl = "https://redacted.sh"; // Default to RED
        MinimumSeeders = 1;
        UseTokenAuth = true;
    }

    public new string ApiKey { get; set; } = string.Empty;
    public bool UseTokenAuth { get; set; }
    public string Username { get; set; } = string.Empty;
    public string Password { get; set; } = string.Empty;
    public bool PreferFlac { get; set; } = true;
    public bool PreferLogScored { get; set; } = true;
    public bool RequireCue { get; set; }
    public int MinimumLogScore { get; set; } = 95;
}
