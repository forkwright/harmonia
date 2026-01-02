// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using Mouseion.Common.Exceptions;

namespace Mouseion.Common.Http
{
    public class InvalidRequestException : MouseionException
    {
        public InvalidRequestException(string message, params object[] args)
            : base(message, args)
        {
        }

        public InvalidRequestException(string message)
            : base(message)
        {
        }

        public InvalidRequestException(string message, Exception innerException)
            : base(message, innerException)
        {
        }
    }
}
