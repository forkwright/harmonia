// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Tags.AutoTagging;

public interface IAutoTaggingRuleRepository : IBasicRepository<AutoTaggingRule>
{
    Task<List<AutoTaggingRule>> GetEnabledRulesAsync(CancellationToken ct = default);
    Task<List<AutoTaggingRule>> GetEnabledRulesForMediaTypeAsync(MediaType mediaType, CancellationToken ct = default);
    Task<List<AutoTaggingRule>> GetRulesByTagIdAsync(int tagId, CancellationToken ct = default);

    List<AutoTaggingRule> GetEnabledRules();
    List<AutoTaggingRule> GetEnabledRulesForMediaType(MediaType mediaType);
    List<AutoTaggingRule> GetRulesByTagId(int tagId);
}
