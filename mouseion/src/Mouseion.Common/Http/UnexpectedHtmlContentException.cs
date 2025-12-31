// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Common.Http
{
    public class UnexpectedHtmlContentException : HttpException
    {
        public UnexpectedHtmlContentException(HttpResponse response)
            : base(response.Request, response, $"Site responded with browser content instead of api data. This disruption may be temporary, please try again later. [{response.Request.Url}]")
        {
        }
    }
}
