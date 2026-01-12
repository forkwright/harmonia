// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Datastore;
using Mouseion.Core.MediaTypes;

namespace Mouseion.Core.Tags.AutoTagging;

public class AutoTaggingRule : ModelBase
{
    public string Name { get; set; } = string.Empty;
    public bool Enabled { get; set; } = true;
    public AutoTaggingConditionType ConditionType { get; set; }
    public string ConditionValue { get; set; } = string.Empty;
    public int TagId { get; set; }
    public MediaType? MediaTypeFilter { get; set; }
}
