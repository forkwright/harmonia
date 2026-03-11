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
    public class HttpException : Exception
    {
        public HttpRequest Request { get; private set; }
        public HttpResponse Response { get; private set; }

        public HttpException(HttpRequest request, HttpResponse response, string message)
            : base(message)
        {
            Request = request;
            Response = response;
        }

        public HttpException(HttpRequest request, HttpResponse response)
            : this(request, response, string.Format("HTTP request failed: [{0}:{1}] [{2}] at [{3}]", (int)response.StatusCode, response.StatusCode, request.Method, request.Url))
        {
        }

        public HttpException(HttpResponse response)
            : this(response.Request, response)
        {
        }

        public override string ToString()
        {
            if (Response != null)
            {
                return base.ToString() + Environment.NewLine + Response.Content;
            }

            return base.ToString();
        }
    }
}
