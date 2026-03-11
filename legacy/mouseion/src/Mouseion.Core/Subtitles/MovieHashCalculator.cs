// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Subtitles;

public static class MovieHashCalculator
{
    private const int HashChunkSize = 65536; // 64KB

    public static string ComputeHash(string filePath)
    {
        var fileInfo = new FileInfo(filePath);
        if (!fileInfo.Exists)
        {
            throw new FileNotFoundException("File not found for hash calculation", filePath);
        }

        var fileSize = fileInfo.Length;
        if (fileSize < HashChunkSize)
        {
            throw new ArgumentException("File is too small for hash calculation (must be >= 64KB)", nameof(filePath));
        }

        ulong hash = (ulong)fileSize;

        using var stream = File.OpenRead(filePath);
        hash += ReadChunk(stream, 0);
        hash += ReadChunk(stream, Math.Max(0, fileSize - HashChunkSize));

        return hash.ToString("x16");
    }

    private static ulong ReadChunk(Stream stream, long position)
    {
        stream.Position = position;
        var buffer = new byte[HashChunkSize];
        var bytesRead = stream.Read(buffer, 0, HashChunkSize);

        ulong hash = 0;
        for (int i = 0; i < bytesRead; i += 8)
        {
            hash += BitConverter.ToUInt64(buffer, i);
        }

        return hash;
    }
}
