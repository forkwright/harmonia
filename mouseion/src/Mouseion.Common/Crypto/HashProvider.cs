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
        byte[] ComputeMd5(string path);
    }

    public class HashProvider : IHashProvider
    {
        public byte[] ComputeMd5(string path)
        {
            using (var md5 = MD5.Create())
            {
                using (var stream = File.OpenRead(path))
                {
                    return md5.ComputeHash(stream);
                }
            }
        }
    }
}
