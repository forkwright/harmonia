// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Audiobooks;
using Mouseion.Core.Books;

namespace Mouseion.Core.Indexers.MyAnonamouse;

public interface IMyAnonamouseIndexer
{
    string Name { get; }
    bool Enabled { get; }
    Task<List<IndexerResult>> SearchBooksAsync(BookSearchCriteria criteria, CancellationToken ct = default);
    Task<List<IndexerResult>> SearchAudiobooksAsync(AudiobookSearchCriteria criteria, CancellationToken ct = default);
}
