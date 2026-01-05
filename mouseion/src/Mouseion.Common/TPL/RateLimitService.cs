// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Concurrent;
using System.Threading.Tasks;
using Mouseion.Common.Cache;
using Mouseion.Common.Extensions;
using Serilog;

namespace Mouseion.Common.TPL
{
    public interface IRateLimitService
    {
        Task WaitAndPulseAsync(string key, TimeSpan interval);
        Task WaitAndPulseAsync(string key, string? subKey, TimeSpan interval);
    }

    public class RateLimitService : IRateLimitService
    {
        private readonly ConcurrentDictionary<string, DateTime> _rateLimitStore;
        private readonly ILogger _logger;

        public RateLimitService(ICacheManager cacheManager, ILogger logger)
        {
            _rateLimitStore = cacheManager.GetCache<ConcurrentDictionary<string, DateTime>>(GetType(), "rateLimitStore").Get("rateLimitStore", () => new ConcurrentDictionary<string, DateTime>());
            _logger = logger;
        }

        public async Task WaitAndPulseAsync(string key, TimeSpan interval)
        {
            await WaitAndPulseAsync(key, null, interval);
        }

        public async Task WaitAndPulseAsync(string key, string? subKey, TimeSpan interval)
        {
            var delay = GetDelay(key, subKey, interval);

            if (delay.TotalSeconds > 0.0)
            {
                _logger.Verbose("Rate Limit triggered, delaying '{Key}' for {Delay:0.000} sec", key, delay.TotalSeconds);
                await Task.Delay(delay);
            }
        }

        private TimeSpan GetDelay(string key, string? subKey, TimeSpan interval)
        {
            var waitUntil = DateTime.UtcNow.Add(interval);

            if (subKey.IsNotNullOrWhiteSpace())
            {
                // Expand the base key timer, but don't extend it beyond now+interval.
                var baseUntil = _rateLimitStore.AddOrUpdate(key,
                    (s) => waitUntil,
                    (s, i) => new DateTime(Math.Max(waitUntil.Ticks, i.Ticks), DateTimeKind.Utc));

                if (baseUntil > waitUntil)
                {
                    waitUntil = baseUntil;
                }

                // Wait for the full key
                var combinedKey = key + "-" + subKey;
                waitUntil = _rateLimitStore.AddOrUpdate(combinedKey,
                    (s) => waitUntil,
                    (s, i) => new DateTime(Math.Max(waitUntil.Ticks, i.Add(interval).Ticks), DateTimeKind.Utc));
            }
            else
            {
                waitUntil = _rateLimitStore.AddOrUpdate(key,
                    (s) => waitUntil,
                    (s, i) => new DateTime(Math.Max(waitUntil.Ticks, i.Add(interval).Ticks), DateTimeKind.Utc));
            }

            waitUntil -= interval;

            return waitUntil - DateTime.UtcNow;
        }
    }
}
