// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;

namespace Mouseion.Common.Exceptions
{
    public class MouseionStartupException : MouseionException
    {
        public MouseionStartupException(string message, params object[] args)
            : base("Mouseion failed to start: " + string.Format(message, args))
        {
        }

        public MouseionStartupException(string message)
            : base("Mouseion failed to start: " + message)
        {
        }

        public MouseionStartupException()
            : base("Mouseion failed to start")
        {
        }

        public MouseionStartupException(Exception innerException, string message, params object[] args)
            : base("Mouseion failed to start: " + string.Format(message, args), innerException)
        {
        }

        public MouseionStartupException(Exception innerException, string message)
            : base("Mouseion failed to start: " + message, innerException)
        {
        }

        public MouseionStartupException(Exception innerException)
            : base("Mouseion failed to start: " + innerException.Message)
        {
        }
    }
}
