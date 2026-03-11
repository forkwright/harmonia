// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using Microsoft.Extensions.Logging;
using Mouseion.Common.Disk;
using Mouseion.Common.EnvironmentInfo;
using Mouseion.Common.Http;

namespace Mouseion.Core.MediaCovers;

public interface IMediaCoverService
{
    string GetCoverPath(int mediaItemId, MediaCoverType coverType, int? height = null);
    Dictionary<string, FileInfo> GetCoverFileInfos();
    void ConvertToLocalUrls(int mediaItemId, IEnumerable<MediaCover> covers, Dictionary<string, FileInfo>? fileInfos = null);
    Task<bool> EnsureCoversAsync(int mediaItemId, IEnumerable<MediaCover> covers);
    void DeleteCovers(int mediaItemId);
}

public class MediaCoverService : IMediaCoverService
{
    private readonly IMediaCoverProxy _mediaCoverProxy;
    private readonly IImageResizer _resizer;
    private readonly IHttpClient _httpClient;
    private readonly IDiskProvider _diskProvider;
    private readonly ICoverExistsSpecification _coverExistsSpecification;
    private readonly ILogger<MediaCoverService> _logger;
    private readonly string _coverRootFolder;

    // ImageSharp is slow on ARM (no hardware acceleration)
    // So limit concurrent resizing tasks
    private static readonly SemaphoreSlim _semaphore = new((int)Math.Ceiling(Environment.ProcessorCount / 2.0));

    public MediaCoverService(
        IMediaCoverProxy mediaCoverProxy,
        IImageResizer resizer,
        IHttpClient httpClient,
        IDiskProvider diskProvider,
        IAppFolderInfo appFolderInfo,
        ICoverExistsSpecification coverExistsSpecification,
        ILogger<MediaCoverService> logger)
    {
        _mediaCoverProxy = mediaCoverProxy;
        _resizer = resizer;
        _httpClient = httpClient;
        _diskProvider = diskProvider;
        _coverExistsSpecification = coverExistsSpecification;
        _logger = logger;
        _coverRootFolder = appFolderInfo.GetMediaCoverPath();
    }

    public string GetCoverPath(int mediaItemId, MediaCoverType coverType, int? height = null)
    {
        var heightSuffix = height.HasValue ? "-" + height.ToString() : "";
        return Path.Combine(GetMediaItemCoverPath(mediaItemId),
            coverType.ToString().ToLower() + heightSuffix + GetExtension(coverType));
    }

    public Dictionary<string, FileInfo> GetCoverFileInfos()
    {
        if (!_diskProvider.FolderExists(_coverRootFolder))
        {
            return new Dictionary<string, FileInfo>();
        }

        return _diskProvider
            .GetFiles(_coverRootFolder, true)
            .Select(f => new FileInfo(f))
            .ToDictionary(x => x.FullName, StringComparer.OrdinalIgnoreCase);
    }

    public void ConvertToLocalUrls(int mediaItemId, IEnumerable<MediaCover> covers, Dictionary<string, FileInfo>? fileInfos = null)
    {
        if (mediaItemId == 0)
        {
            ConvertToProxyUrls(covers);
        }
        else
        {
            ConvertToMediaCoverUrls(mediaItemId, covers, fileInfos);
        }
    }

    private void ConvertToProxyUrls(IEnumerable<MediaCover> covers)
    {
        foreach (var mediaCover in covers)
        {
            mediaCover.Url = _mediaCoverProxy.RegisterUrl(mediaCover.RemoteUrl);
        }
    }

    private void ConvertToMediaCoverUrls(int mediaItemId, IEnumerable<MediaCover> covers, Dictionary<string, FileInfo>? fileInfos)
    {
        foreach (var mediaCover in covers)
        {
            if (mediaCover.CoverType == MediaCoverType.Unknown)
            {
                continue;
            }

            SetMediaCoverUrl(mediaItemId, mediaCover, fileInfos);
        }
    }

    private void SetMediaCoverUrl(int mediaItemId, MediaCover mediaCover, Dictionary<string, FileInfo>? fileInfos)
    {
        var filePath = GetCoverPath(mediaItemId, mediaCover.CoverType);
        mediaCover.Url = $"/MediaCover/{mediaItemId}/{mediaCover.CoverType.ToString().ToLower()}{GetExtension(mediaCover.CoverType)}";

        var lastWrite = GetLastWriteTime(filePath, fileInfos);
        if (lastWrite.HasValue)
        {
            mediaCover.Url += "?lastWrite=" + lastWrite.Value.Ticks;
        }
    }

    private DateTime? GetLastWriteTime(string filePath, Dictionary<string, FileInfo>? fileInfos)
    {
        if (fileInfos != null && fileInfos.TryGetValue(filePath, out var file))
        {
            return file.LastWriteTimeUtc;
        }

        if (_diskProvider.FileExists(filePath))
        {
            return _diskProvider.FileGetLastWrite(filePath);
        }

        return null;
    }

    public async Task<bool> EnsureCoversAsync(int mediaItemId, IEnumerable<MediaCover> covers)
    {
        var updated = false;
        var toResize = new List<Tuple<MediaCover, bool>>();

        foreach (var cover in covers)
        {
            if (cover.CoverType == MediaCoverType.Unknown)
            {
                continue;
            }

            var fileName = GetCoverPath(mediaItemId, cover.CoverType);
            var alreadyExists = false;

            try
            {
                alreadyExists = await _coverExistsSpecification.AlreadyExistsAsync(cover.RemoteUrl, fileName).ConfigureAwait(false);

                if (!alreadyExists)
                {
                    await DownloadCoverAsync(mediaItemId, cover);
                    updated = true;
                }
            }
            catch (HttpException ex)
            {
                _logger.LogWarning(ex, "Couldn't download media cover for item {MediaItemId}: {Message}", mediaItemId, ex.Message);
            }
            catch (IOException ex)
            {
                _logger.LogError(ex, "I/O error downloading media cover for item {MediaItemId}", mediaItemId);
            }
            catch (HttpRequestException ex)
            {
                _logger.LogError(ex, "Network error downloading media cover for item {MediaItemId}", mediaItemId);
            }

            toResize.Add(Tuple.Create(cover, alreadyExists));
        }

        try
        {
            await _semaphore.WaitAsync();

            foreach (var tuple in toResize)
            {
                EnsureResizedCovers(mediaItemId, tuple.Item1, !tuple.Item2);
            }
        }
        finally
        {
            _semaphore.Release();
        }

        return updated;
    }

    public void DeleteCovers(int mediaItemId)
    {
        var path = GetMediaItemCoverPath(mediaItemId);
        if (_diskProvider.FolderExists(path))
        {
            _diskProvider.DeleteFolder(path, true);
        }
    }

    private string GetMediaItemCoverPath(int mediaItemId)
    {
        return Path.Combine(_coverRootFolder, mediaItemId.ToString());
    }

    private async Task DownloadCoverAsync(int mediaItemId, MediaCover cover)
    {
        var fileName = GetCoverPath(mediaItemId, cover.CoverType);
        _logger.LogInformation("Downloading {CoverType} for media item {MediaItemId} from {Url}",
            cover.CoverType, mediaItemId, cover.RemoteUrl);

        _diskProvider.EnsureFolder(Path.GetDirectoryName(fileName)!);
        await _httpClient.DownloadFileAsync(cover.RemoteUrl, fileName);
    }

    private void EnsureResizedCovers(int mediaItemId, MediaCover cover, bool forceResize)
    {
        int[] heights = cover.CoverType switch
        {
            MediaCoverType.Poster or MediaCoverType.Headshot => new[] { 500, 250 },
            MediaCoverType.Banner => new[] { 70, 35 },
            MediaCoverType.Fanart or MediaCoverType.Screenshot => new[] { 360, 180 },
            _ => Array.Empty<int>()
        };

        foreach (var height in heights)
        {
            var mainFileName = GetCoverPath(mediaItemId, cover.CoverType);
            var resizeFileName = GetCoverPath(mediaItemId, cover.CoverType, height);

            if (forceResize || !_diskProvider.FileExists(resizeFileName) || _diskProvider.GetFileSize(resizeFileName) == 0)
            {
                _logger.LogDebug("Resizing {CoverType}-{Height} for media item {MediaItemId}",
                    cover.CoverType, height, mediaItemId);

                try
                {
                    _resizer.Resize(mainFileName, resizeFileName, height);
                }
                catch (InvalidOperationException ex)
                {
                    _logger.LogDebug(ex, "Couldn't resize media cover {CoverType}-{Height} for item {MediaItemId}, using full size instead (invalid operation)",
                        cover.CoverType, height, mediaItemId);
                }
                catch (IOException ex)
                {
                    _logger.LogDebug(ex, "Couldn't resize media cover {CoverType}-{Height} for item {MediaItemId}, using full size instead (I/O error)",
                        cover.CoverType, height, mediaItemId);
                }
            }
        }
    }

    private static string GetExtension(MediaCoverType coverType)
    {
        return coverType switch
        {
            MediaCoverType.Clearlogo => ".png",
            _ => ".jpg"
        };
    }
}
