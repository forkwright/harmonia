// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System.Security.Cryptography;
using System.Text;
using Mouseion.Common.Cache;
using Mouseion.Common.Http;

namespace Mouseion.Core.MediaCovers;

public interface IMediaCoverProxy
{
    string RegisterUrl(string url);
    string GetUrl(string hash);
    Task<byte[]> GetImageAsync(string hash);
}

public class MediaCoverProxy : IMediaCoverProxy
{
    private readonly IHttpClient _httpClient;
    private readonly ICached<string> _cache;

    public MediaCoverProxy(IHttpClient httpClient, ICacheManager cacheManager)
    {
        _httpClient = httpClient;
        _cache = cacheManager.GetCache<string>(GetType());
    }

    public string RegisterUrl(string url)
    {
        if (string.IsNullOrWhiteSpace(url))
        {
            return string.Empty;
        }

        var hash = ComputeSha256Hash(url);
        _cache.Set(hash, url, TimeSpan.FromHours(24));
        _cache.ClearExpired();

        var fileName = Path.GetFileName(url);
        return $"/MediaCoverProxy/{hash}/{fileName}";
    }

    public string GetUrl(string hash)
    {
        var result = _cache.Find(hash);
        if (result == null)
        {
            throw new KeyNotFoundException("URL no longer in cache");
        }

        return result;
    }

    public async Task<byte[]> GetImageAsync(string hash)
    {
        var url = GetUrl(hash);
        var request = new HttpRequest(url);
        var response = await _httpClient.GetAsync(request);
        return response.ResponseData;
    }

    private static string ComputeSha256Hash(string input)
    {
        var bytes = SHA256.HashData(Encoding.UTF8.GetBytes(input));
        var builder = new StringBuilder();
        foreach (var b in bytes)
        {
            builder.Append(b.ToString("x2"));
        }

        return builder.ToString();
    }
}
