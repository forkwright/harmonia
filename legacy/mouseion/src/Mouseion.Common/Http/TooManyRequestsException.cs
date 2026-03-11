// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;

namespace Mouseion.Common.Http
{
    public class TooManyRequestsException : HttpException
    {
        public TimeSpan RetryAfter { get; private set; }

        public TooManyRequestsException(HttpRequest request, HttpResponse response)
            : base(request, response)
        {
            if (response.Headers.ContainsKey("Retry-After"))
            {
                var retryAfter = response.Headers["Retry-After"]?.ToString();

                if (retryAfter != null && int.TryParse(retryAfter, out var seconds))
                {
                    RetryAfter = TimeSpan.FromSeconds(seconds);
                }
                else if (retryAfter != null && DateTime.TryParse(retryAfter, out var date))
                {
                    RetryAfter = date.ToUniversalTime() - DateTime.UtcNow;
                }
            }
        }
    }
}
