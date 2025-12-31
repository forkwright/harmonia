// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Diagnostics;

namespace Mouseion.Common.EnsureThat
{
    public static class EnsureNullableValueTypeExtensions
    {
        [DebuggerStepThrough]
        public static Param<T?> IsNotNull<T>(this Param<T?> param)
            where T : struct
        {
            if (param.Value == null || !param.Value.HasValue)
            {
                throw ExceptionFactory.CreateForParamNullValidation(param.Name, ExceptionMessages.EnsureExtensions_IsNotNull);
            }

            return param;
        }
    }
}
