// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.IO;
using System.Reflection;

namespace Mouseion.Common.Extensions
{
    public static class ResourceExtensions
    {
        public static byte[] GetManifestResourceBytes(this Assembly assembly, string name)
        {
            var stream = assembly.GetManifestResourceStream(name);

            var result = new byte[stream.Length];
            var read = stream.Read(result, 0, result.Length);

            if (read != result.Length)
            {
                throw new EndOfStreamException("Reached end of stream before reading enough bytes.");
            }

            return result;
        }
    }
}
