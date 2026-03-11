// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.IO;

namespace Mouseion.Common.Disk
{
    public class DestinationAlreadyExistsException : IOException
    {
        public DestinationAlreadyExistsException()
        {
        }

        public DestinationAlreadyExistsException(string message)
            : base(message)
        {
        }

        public DestinationAlreadyExistsException(string message, int hresult)
            : base(message, hresult)
        {
        }

        public DestinationAlreadyExistsException(string message, Exception innerException)
            : base(message, innerException)
        {
        }
    }
}
