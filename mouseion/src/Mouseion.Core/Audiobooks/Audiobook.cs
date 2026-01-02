// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Text.Json;
using Mouseion.Core.MediaItems;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Audiobooks;

public class Audiobook : MediaItem
{
    private AudiobookMetadata? _metadata;

    public Audiobook()
    {
        MediaType = MediaType.Audiobook;
        _metadata = new AudiobookMetadata();
    }

    public string Title { get; set; } = null!;
    public int Year { get; set; }

    // Metadata stored in JSON column
    public AudiobookMetadata Metadata
    {
        get => _metadata ??= new AudiobookMetadata();
        set => _metadata = value;
    }

    // Serialized JSON for database storage
    public string? AudiobookMetadata
    {
        get => Metadata != null ? JsonSerializer.Serialize(Metadata) : null;
        set => _metadata = value != null ? JsonSerializer.Deserialize<AudiobookMetadata>(value) : null;
    }

    public override string GetTitle() => Title;
    public override int GetYear() => Year;

    public override string ToString()
    {
        var narratorInfo = string.IsNullOrEmpty(Metadata?.Narrator) ? "" : $" - Narrated by {Metadata.Narrator}";
        return $"{Title} ({Year}){narratorInfo}";
    }
}
