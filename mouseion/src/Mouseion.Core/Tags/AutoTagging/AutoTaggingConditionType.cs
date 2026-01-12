// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.Tags.AutoTagging;

public enum AutoTaggingConditionType
{
    GenreContains = 0,
    LanguageContains = 1,
    QualityEquals = 2,
    QualityGroupEquals = 3,
    FormatEquals = 4,
    BitDepthAtLeast = 5,
    Custom = 99
}
