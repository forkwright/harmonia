// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Core.Movies.Organization;

namespace Mouseion.Core.MediaFiles.Import;

public class ImportSettings
{
    public FileStrategy DefaultStrategy { get; set; } = FileStrategy.Hardlink;
    public bool VerifyChecksum { get; set; } = true;
    public bool PreserveTimestamps { get; set; } = true;
    public List<FileStrategy> AvailableStrategies { get; set; } = new()
    {
        FileStrategy.Hardlink,
        FileStrategy.Copy,
        FileStrategy.Move
    };
}
