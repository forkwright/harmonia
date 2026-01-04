// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Mouseion.Common.Disk;
using SixLabors.ImageSharp;
using SixLabors.ImageSharp.Formats.Jpeg;
using SixLabors.ImageSharp.Processing;

namespace Mouseion.Core.MediaCovers;

public interface IImageResizer
{
    void Resize(string source, string destination, int height);
}

public class ImageResizer : IImageResizer
{
    private readonly IDiskProvider _diskProvider;

    public ImageResizer(IDiskProvider diskProvider)
    {
        _diskProvider = diskProvider;

        // Thumbnails don't need super high quality
        SixLabors.ImageSharp.Configuration.Default.ImageFormatsManager.SetEncoder(JpegFormat.Instance, new JpegEncoder
        {
            Quality = 92
        });
    }

    public void Resize(string source, string destination, int height)
    {
        try
        {
            using var image = Image.Load(source);
            image.Mutate(x => x.Resize(0, height));
            image.Save(destination);
        }
        catch
        {
            if (_diskProvider.FileExists(destination))
            {
                _diskProvider.DeleteFile(destination);
            }

            throw;
        }
    }
}
