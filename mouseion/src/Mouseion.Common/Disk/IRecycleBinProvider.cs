// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Common.Disk;

public interface IRecycleBinProvider
{
    /// <summary>
    /// Deletes a file by moving it to the recycle bin/trash.
    /// </summary>
    /// <param name="path">The file path to delete</param>
    /// <returns>True if successful, false otherwise</returns>
    bool DeleteFile(string path);

    /// <summary>
    /// Deletes a folder by moving it to the recycle bin/trash.
    /// </summary>
    /// <param name="path">The folder path to delete</param>
    /// <returns>True if successful, false otherwise</returns>
    bool DeleteFolder(string path);

    /// <summary>
    /// Checks if recycle bin is available on this platform.
    /// </summary>
    bool IsAvailable { get; }

    /// <summary>
    /// Gets the path to the recycle bin/trash directory (if accessible).
    /// </summary>
    string? GetRecycleBinPath();
}
