// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;

namespace Mouseion.Core.Authors;

public interface IAddAuthorService
{
    Task<Author> AddAuthorAsync(Author author, CancellationToken ct = default);
    Task<List<Author>> AddAuthorsAsync(List<Author> authors, CancellationToken ct = default);

    Author AddAuthor(Author author);
    List<Author> AddAuthors(List<Author> authors);
}

public class AddAuthorService : IAddAuthorService
{
    private readonly IAuthorRepository _authorRepository;
    private readonly ILogger<AddAuthorService> _logger;

    public AddAuthorService(IAuthorRepository authorRepository, ILogger<AddAuthorService> logger)
    {
        _authorRepository = authorRepository;
        _logger = logger;
    }

    public async Task<Author> AddAuthorAsync(Author author, CancellationToken ct = default)
    {
        ValidateAuthor(author);

        // Check for existing author by foreign ID
        if (!string.IsNullOrWhiteSpace(author.ForeignAuthorId))
        {
            var existing = await _authorRepository.FindByForeignIdAsync(author.ForeignAuthorId, ct).ConfigureAwait(false);
            if (existing != null)
            {
                _logger.LogInformation("Author already exists: {AuthorName} (ID: {ForeignId})",
                    existing.Name, existing.ForeignAuthorId);
                return existing;
            }
        }

        // Check for existing author by name (fallback)
        var existingByName = await _authorRepository.FindByNameAsync(author.Name, ct).ConfigureAwait(false);
        if (existingByName != null)
        {
            _logger.LogWarning("Author with same name already exists: {AuthorName}", existingByName.Name);
            return existingByName;
        }

        // Set defaults
        if (string.IsNullOrWhiteSpace(author.SortName))
        {
            author.SortName = GenerateSortName(author.Name);
        }

        author.Added = DateTime.UtcNow;

        var added = await _authorRepository.InsertAsync(author, ct).ConfigureAwait(false);
        _logger.LogInformation("Added author: {AuthorName} (ID: {AuthorId})", added.Name, added.Id);

        return added;
    }

    public Author AddAuthor(Author author)
    {
        ValidateAuthor(author);

        // Check for existing author by foreign ID
        if (!string.IsNullOrWhiteSpace(author.ForeignAuthorId))
        {
            var existing = _authorRepository.FindByForeignId(author.ForeignAuthorId);
            if (existing != null)
            {
                _logger.LogInformation("Author already exists: {AuthorName} (ID: {ForeignId})",
                    existing.Name, existing.ForeignAuthorId);
                return existing;
            }
        }

        // Check for existing author by name (fallback)
        var existingByName = _authorRepository.FindByName(author.Name);
        if (existingByName != null)
        {
            _logger.LogWarning("Author with same name already exists: {AuthorName}", existingByName.Name);
            return existingByName;
        }

        // Set defaults
        if (string.IsNullOrWhiteSpace(author.SortName))
        {
            author.SortName = GenerateSortName(author.Name);
        }

        author.Added = DateTime.UtcNow;

        var added = _authorRepository.Insert(author);
        _logger.LogInformation("Added author: {AuthorName} (ID: {AuthorId})", added.Name, added.Id);

        return added;
    }

    public async Task<List<Author>> AddAuthorsAsync(List<Author> authors, CancellationToken ct = default)
    {
        var addedAuthors = new List<Author>();

        foreach (var author in authors)
        {
            try
            {
                var added = await AddAuthorAsync(author, ct).ConfigureAwait(false);
                addedAuthors.Add(added);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error adding author: {AuthorName}", author.Name);
            }
        }

        return addedAuthors;
    }

    public List<Author> AddAuthors(List<Author> authors)
    {
        var addedAuthors = new List<Author>();

        foreach (var author in authors)
        {
            try
            {
                var added = AddAuthor(author);
                addedAuthors.Add(added);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error adding author: {AuthorName}", author.Name);
            }
        }

        return addedAuthors;
    }

    private static void ValidateAuthor(Author author)
    {
        if (string.IsNullOrWhiteSpace(author.Name))
        {
            throw new ArgumentException("Author name is required", nameof(author));
        }

        if (author.QualityProfileId <= 0)
        {
            throw new ArgumentException("Quality profile ID must be set", nameof(author));
        }
    }

    private static string GenerateSortName(string name)
    {
        // Simple sort name generation (move "The", "A", "An" to end)
        var prefixes = new[] { "The ", "A ", "An " };

        foreach (var prefix in prefixes)
        {
            if (name.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
            {
                return $"{name[prefix.Length..]}, {prefix.TrimEnd()}";
            }
        }

        return name;
    }
}
