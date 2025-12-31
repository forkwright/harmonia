// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Common.Options
{
    public class AuthOptions
    {
        public string? ApiKey { get; set; }
        public bool? Enabled { get; set; }
        public string? Method { get; set; }
        public string? Required { get; set; }
        public bool? TrustCgnatIpAddresses { get; set; }
    }
}
