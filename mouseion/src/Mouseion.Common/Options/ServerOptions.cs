// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Common.Options
{
    public class ServerOptions
    {
        public string? UrlBase { get; set; }
        public string? BindAddress { get; set; }
        public int? Port { get; set; }
        public bool? EnableSsl { get; set; }
        public int? SslPort { get; set; }
        public string? SslCertPath { get; set; }
        public string? SslCertPassword { get; set; }
    }
}
