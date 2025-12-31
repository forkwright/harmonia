// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Security;

namespace Mouseion.Common.Security
{
    public interface IPathValidator
    {
        string ValidateAndNormalizePath(string path, string baseDirectory);
        bool IsPathSafe(string path, string baseDirectory);
    }

    public class PathValidator : IPathValidator
    {
        public string ValidateAndNormalizePath(string path, string baseDirectory)
        {
            if (string.IsNullOrWhiteSpace(path))
            {
                throw new ArgumentException("Path cannot be null or empty", nameof(path));
            }

            if (string.IsNullOrWhiteSpace(baseDirectory))
            {
                throw new ArgumentException("Base directory cannot be null or empty", nameof(baseDirectory));
            }

            // Normalize paths
            var normalizedBase = Path.GetFullPath(baseDirectory);
            var normalizedPath = Path.GetFullPath(Path.Combine(baseDirectory, path));

            // Ensure the resolved path is within the base directory
            if (!normalizedPath.StartsWith(normalizedBase, StringComparison.OrdinalIgnoreCase))
            {
                throw new SecurityException($"Access denied: Path '{path}' is outside the allowed directory");
            }

            return normalizedPath;
        }

        public bool IsPathSafe(string path, string baseDirectory)
        {
            try
            {
                ValidateAndNormalizePath(path, baseDirectory);
                return true;
            }
            catch
            {
                return false;
            }
        }
    }
}
