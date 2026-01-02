// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Core.Authors;
using Mouseion.Core.MediaItems;

namespace Mouseion.Core.Audiobooks;

public interface IAddAudiobookService
{
    Task<Audiobook> AddAudiobookAsync(Audiobook audiobook, CancellationToken ct = default);
    Task<List<Audiobook>> AddAudiobooksAsync(List<Audiobook> audiobooks, CancellationToken ct = default);

    Audiobook AddAudiobook(Audiobook audiobook);
    List<Audiobook> AddAudiobooks(List<Audiobook> audiobooks);
}

public class AddAudiobookService : AddMediaItemService<Audiobook, IAudiobookRepository>, IAddAudiobookService
{
    private readonly IAudiobookRepository _audiobookRepository;

    public AddAudiobookService(
        IAudiobookRepository audiobookRepository,
        IAuthorRepository authorRepository,
        ILogger<AddAudiobookService> logger)
        : base(audiobookRepository, authorRepository, logger)
    {
        _audiobookRepository = audiobookRepository;
    }

    public Task<Audiobook> AddAudiobookAsync(Audiobook audiobook, CancellationToken ct = default)
        => AddItemAsync(audiobook, ct);

    public Audiobook AddAudiobook(Audiobook audiobook)
        => AddItem(audiobook);

    public Task<List<Audiobook>> AddAudiobooksAsync(List<Audiobook> audiobooks, CancellationToken ct = default)
        => AddItemsAsync(audiobooks, ct);

    public List<Audiobook> AddAudiobooks(List<Audiobook> audiobooks)
        => AddItems(audiobooks);

    protected override Task<Audiobook?> FindByTitleAsync(string title, int year, CancellationToken ct = default)
        => _audiobookRepository.FindByTitleAsync(title, year, ct);

    protected override Audiobook? FindByTitle(string title, int year)
        => _audiobookRepository.FindByTitle(title, year);

    protected override void LogItemAdded(Audiobook audiobook)
        => Logger.LogInformation("Added audiobook: {AudiobookTitle} ({Year}) - Narrator: {Narrator}, Author ID: {AuthorId}",
            audiobook.Title, audiobook.Year, audiobook.Metadata.Narrator, audiobook.AuthorId);

    protected override void LogItemExists(Audiobook audiobook)
        => Logger.LogInformation("Audiobook already exists: {AudiobookTitle} ({Year}) - Narrator: {Narrator}",
            audiobook.Title, audiobook.Year, audiobook.Metadata.Narrator);
}
