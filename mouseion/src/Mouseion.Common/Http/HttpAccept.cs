// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Common.Http
{
    public sealed class HttpAccept
    {
        public static readonly HttpAccept Rss = new HttpAccept("application/rss+xml, text/rss+xml, application/xml, text/xml");
        public static readonly HttpAccept Json = new HttpAccept("application/json");
        public static readonly HttpAccept JsonCharset = new HttpAccept("application/json; charset=utf-8");
        public static readonly HttpAccept Html = new HttpAccept("text/html");

        public string Value { get; private set; }

        public HttpAccept(string accept)
        {
            Value = accept;
        }

        public override string ToString()
        {
            return Value;
        }
    }
}
