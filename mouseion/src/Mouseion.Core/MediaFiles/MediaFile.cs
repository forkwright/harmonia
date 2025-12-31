// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;

namespace Mouseion.Core.MediaFiles
{
    public class MediaFile : ModelBase
    {
        public int MediaItemId { get; set; }
        public MediaType MediaType { get; set; }
        public string Path { get; set; } = string.Empty;
        public string? RelativePath { get; set; }
        public long Size { get; set; }
        public DateTime DateAdded { get; set; }
        public int? DurationSeconds { get; set; }
        public int? Bitrate { get; set; }
        public int? SampleRate { get; set; }
        public int? Channels { get; set; }
        public string? Format { get; set; }
        public string? Quality { get; set; }

        public override string ToString()
        {
            return $"[{Id}] {RelativePath}";
        }

        public string GetFileName()
        {
            if (!string.IsNullOrWhiteSpace(RelativePath))
            {
                return System.IO.Path.GetFileNameWithoutExtension(RelativePath);
            }

            if (!string.IsNullOrWhiteSpace(Path))
            {
                return System.IO.Path.GetFileNameWithoutExtension(Path);
            }

            return string.Empty;
        }
    }

    public enum MediaType
    {
        Movie = 0,
        Audiobook = 1,
        Book = 2,
        Music = 3,
        Podcast = 4,
        TVShow = 5
    }
}
