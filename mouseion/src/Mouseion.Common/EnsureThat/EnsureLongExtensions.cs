// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Diagnostics;
using Mouseion.Common.Extensions;

namespace Mouseion.Common.EnsureThat
{
    public static class EnsureLongExtensions
    {
        [DebuggerStepThrough]
        public static Param<long> IsLt(this Param<long> param, long limit)
        {
            if (param.Value >= limit)
            {
                throw ExceptionFactory.CreateForParamValidation(param.Name, ExceptionMessages.EnsureExtensions_IsNotLt.Inject(param.Value, limit));
            }

            return param;
        }

        [DebuggerStepThrough]
        public static Param<long> IsLte(this Param<long> param, long limit)
        {
            if (!(param.Value <= limit))
            {
                throw ExceptionFactory.CreateForParamValidation(param.Name, ExceptionMessages.EnsureExtensions_IsNotLte.Inject(param.Value, limit));
            }

            return param;
        }

        [DebuggerStepThrough]
        public static Param<long> IsGt(this Param<long> param, long limit)
        {
            if (param.Value <= limit)
            {
                throw ExceptionFactory.CreateForParamValidation(param.Name, ExceptionMessages.EnsureExtensions_IsNotGt.Inject(param.Value, limit));
            }

            return param;
        }

        [DebuggerStepThrough]
        public static Param<long> IsGte(this Param<long> param, long limit)
        {
            if (!(param.Value >= limit))
            {
                throw ExceptionFactory.CreateForParamValidation(param.Name, ExceptionMessages.EnsureExtensions_IsNotGte.Inject(param.Value, limit));
            }

            return param;
        }

        [DebuggerStepThrough]
        public static Param<long> IsInRange(this Param<long> param, long min, long max)
        {
            if (param.Value < min)
            {
                throw ExceptionFactory.CreateForParamValidation(param.Name, ExceptionMessages.EnsureExtensions_IsNotInRange_ToLow.Inject(param.Value, min));
            }

            if (param.Value > max)
            {
                throw ExceptionFactory.CreateForParamValidation(param.Name, ExceptionMessages.EnsureExtensions_IsNotInRange_ToHigh.Inject(param.Value, max));
            }

            return param;
        }
    }
}
