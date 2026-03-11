// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Tags;

public class TagService : ITagService
{
    private readonly ITagRepository _repository;

    public TagService(ITagRepository repository)
    {
        _repository = repository;
    }

    public Tag GetTag(int tagId)
    {
        var tag = _repository.Find(tagId);
        if (tag == null)
        {
            throw new InvalidOperationException($"Tag {tagId} not found");
        }
        return tag;
    }

    public Tag GetTag(string tag)
    {
        if (tag.All(char.IsDigit))
        {
            return GetTag(int.Parse(tag));
        }

        var result = _repository.GetByLabel(tag);
        if (result == null)
        {
            throw new InvalidOperationException($"Tag '{tag}' not found");
        }
        return result;
    }

    public List<Tag> GetTags(IEnumerable<int> ids)
    {
        return _repository.GetTags(ids.ToHashSet());
    }

    public List<Tag> All()
    {
        return _repository.All().OrderBy(t => t.Label).ToList();
    }

    public Tag Add(Tag tag)
    {
        var existingTag = _repository.FindByLabel(tag.Label);

        if (existingTag != null)
        {
            return existingTag;
        }

        tag.Label = tag.Label.ToLowerInvariant();
        return _repository.Insert(tag);
    }

    public Tag Update(Tag tag)
    {
        tag.Label = tag.Label.ToLowerInvariant();
        return _repository.Update(tag);
    }

    public void Delete(int tagId)
    {
        _repository.Delete(tagId);
    }

    public async Task<Tag> GetTagAsync(int tagId, CancellationToken ct = default)
    {
        var tag = await _repository.FindAsync(tagId, ct).ConfigureAwait(false);
        if (tag == null)
        {
            throw new InvalidOperationException($"Tag {tagId} not found");
        }
        return tag;
    }

    public async Task<Tag> GetTagAsync(string tag, CancellationToken ct = default)
    {
        if (tag.All(char.IsDigit))
        {
            return await GetTagAsync(int.Parse(tag), ct).ConfigureAwait(false);
        }

        var result = await _repository.GetByLabelAsync(tag, ct).ConfigureAwait(false);
        if (result == null)
        {
            throw new InvalidOperationException($"Tag '{tag}' not found");
        }
        return result;
    }

    public async Task<List<Tag>> GetTagsAsync(IEnumerable<int> ids, CancellationToken ct = default)
    {
        return await _repository.GetTagsAsync(ids.ToHashSet(), ct).ConfigureAwait(false);
    }

    public async Task<List<Tag>> AllAsync(CancellationToken ct = default)
    {
        var tags = await _repository.AllAsync(ct).ConfigureAwait(false);
        return tags.OrderBy(t => t.Label).ToList();
    }

    public async Task<Tag> AddAsync(Tag tag, CancellationToken ct = default)
    {
        var existingTag = await _repository.FindByLabelAsync(tag.Label, ct).ConfigureAwait(false);

        if (existingTag != null)
        {
            return existingTag;
        }

        tag.Label = tag.Label.ToLowerInvariant();
        return await _repository.InsertAsync(tag, ct).ConfigureAwait(false);
    }

    public async Task<Tag> UpdateAsync(Tag tag, CancellationToken ct = default)
    {
        tag.Label = tag.Label.ToLowerInvariant();
        return await _repository.UpdateAsync(tag, ct).ConfigureAwait(false);
    }

    public async Task DeleteAsync(int tagId, CancellationToken ct = default)
    {
        await _repository.DeleteAsync(tagId, ct).ConfigureAwait(false);
    }
}
