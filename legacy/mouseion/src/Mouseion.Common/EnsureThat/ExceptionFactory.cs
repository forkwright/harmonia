// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using Mouseion.Common.Extensions;
using Serilog;

namespace Mouseion.Common.EnsureThat
{
    internal static class ExceptionFactory
    {
        private static readonly ILogger Logger = Log.ForContext(typeof(ExceptionFactory));

        internal static ArgumentException CreateForParamValidation(string paramName, string message)
        {
            Logger.Warning(message.SanitizeForLog());
            return new ArgumentException(message, paramName);
        }

        internal static ArgumentNullException CreateForParamNullValidation(string paramName, string message)
        {
            Logger.Warning(message.SanitizeForLog());
            return new ArgumentNullException(paramName, message);
        }
    }
}
