// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.MediaItems;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Tags.AutoTagging;

public interface IAutoTaggingService
{
    Task<HashSet<int>> EvaluateRulesAsync(MediaItem item, CancellationToken ct = default);
    Task ApplyAutoTagsAsync(MediaItem item, CancellationToken ct = default);
    Task<int> ApplyAutoTagsToAllAsync(MediaType? mediaType, CancellationToken ct = default);
    Task<HashSet<int>> PreviewTagsAsync(MediaItem item, CancellationToken ct = default);

    HashSet<int> EvaluateRules(MediaItem item);
    void ApplyAutoTags(MediaItem item);
}
