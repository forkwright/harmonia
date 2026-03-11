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

namespace Mouseion.Core.Books;

public class Book : MediaItem
{
    private BookMetadata? _metadata;

    public Book()
    {
        MediaType = MediaType.Book;
        _metadata = new BookMetadata();
    }

    public string Title { get; set; } = null!;
    public int Year { get; set; }

    // Metadata stored in JSON column
    public BookMetadata Metadata
    {
        get => _metadata ??= new BookMetadata();
        set => _metadata = value;
    }

    // Serialized JSON for database storage
    public string? BookMetadata
    {
        get => Metadata != null ? JsonSerializer.Serialize(Metadata) : null;
        set => _metadata = value != null ? JsonSerializer.Deserialize<BookMetadata>(value) : null;
    }

    public override string GetTitle() => Title;
    public override int GetYear() => Year;

    public override string ToString()
    {
        return $"{Title} ({Year})";
    }
}
