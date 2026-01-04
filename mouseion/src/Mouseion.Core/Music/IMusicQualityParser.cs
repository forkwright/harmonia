// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Qualities;

namespace Mouseion.Core.Music;

public interface IMusicQualityParser
{
    QualityModel ParseQuality(string name);
    bool IsMusicFile(string path);
}
