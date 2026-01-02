// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

namespace Mouseion.Core.MediaFiles.Import;

public class ImportDecision
{
    public MusicFileInfo MusicFileInfo { get; }
    public List<ImportRejection> Rejections { get; }
    public bool Approved => Rejections.Count == 0;

    public ImportDecision(MusicFileInfo musicFileInfo)
    {
        MusicFileInfo = musicFileInfo;
        Rejections = new List<ImportRejection>();
    }

    public ImportDecision(MusicFileInfo musicFileInfo, params ImportRejection[] rejections)
    {
        MusicFileInfo = musicFileInfo;
        Rejections = rejections.ToList();
    }

    public void AddRejection(ImportRejection rejection)
    {
        Rejections.Add(rejection);
    }
}
