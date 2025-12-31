// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;

namespace Mouseion.Common.Exceptions
{
    public abstract class MouseionException : ApplicationException
    {
        protected MouseionException(string message, params object[] args)
            : base(string.Format(message, args))
        {
        }

        protected MouseionException(string message)
            : base(message)
        {
        }

        protected MouseionException(string message, Exception innerException, params object[] args)
            : base(string.Format(message, args), innerException)
        {
        }

        protected MouseionException(string message, Exception innerException)
            : base(message, innerException)
        {
        }
    }
}
