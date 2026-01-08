// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
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
    private readonly ILogger<ImageResizer> _logger;

    public ImageResizer(IDiskProvider diskProvider, ILogger<ImageResizer> logger)
    {
        _diskProvider = diskProvider;
        _logger = logger;

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
#pragma warning disable S2139 // Exceptions should be either logged or rethrown but not both
        catch (Exception ex)
        {
            if (_logger.IsEnabled(LogLevel.Error))
            {
                _logger.LogError(ex, "Failed to resize image from {Source} to {Destination}", source, destination);
            }
            // Exception logged for diagnostic purposes, cleanup performed, then rethrown to caller
            if (_diskProvider.FileExists(destination))
            {
                _diskProvider.DeleteFile(destination);
            }

            throw;
        }
#pragma warning restore S2139
    }
}
