// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Security.Cryptography;
using System.Text;

namespace Mouseion.Common.Crypto
{
    public static class HashConverter
    {
        public static int GetHashInt31(string target)
        {
            var hash = GetHash(target);
            return BitConverter.ToInt32(hash, 0) & 0x7fffffff;
        }

        public static byte[] GetHash(string target)
        {
            using var sha256 = SHA256.Create();
            return sha256.ComputeHash(Encoding.UTF8.GetBytes(target));
        }
    }
}
