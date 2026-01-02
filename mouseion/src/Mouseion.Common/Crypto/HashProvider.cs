// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.IO;
using System.Security.Cryptography;

namespace Mouseion.Common.Crypto
{
    public interface IHashProvider
    {
        byte[] ComputeHash(string path);
    }

    public class HashProvider : IHashProvider
    {
        public byte[] ComputeHash(string path)
        {
            using (var sha256 = SHA256.Create())
            {
                using (var stream = File.OpenRead(path))
                {
                    return sha256.ComputeHash(stream);
                }
            }
        }
    }
}
