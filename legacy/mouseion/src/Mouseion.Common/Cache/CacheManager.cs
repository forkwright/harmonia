// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using Mouseion.Common.EnsureThat;

namespace Mouseion.Common.Cache
{
    public class CacheManager : ICacheManager
    {
        private readonly ConcurrentDictionary<string, ICached> _cache;

        public CacheManager()
        {
            _cache = new ConcurrentDictionary<string, ICached>();
        }

        public ICached<T> GetCache<T>(Type host)
        {
            Ensure.That(host, () => host).IsNotNull();

            return GetCache<T>(host, host.FullName!);
        }

        public ICached<T> GetCache<T>(Type host, string name)
        {
            Ensure.That(host, () => host).IsNotNull();
            Ensure.That(name, () => name).IsNotNullOrWhiteSpace();

            return (ICached<T>)_cache.GetOrAdd(host.FullName + "_" + name, s => new Cached<T>());
        }

        public ICached<T> GetRollingCache<T>(Type host, string name, TimeSpan defaultLifeTime)
        {
            Ensure.That(host, () => host).IsNotNull();
            Ensure.That(name, () => name).IsNotNullOrWhiteSpace();

            return (ICached<T>)_cache.GetOrAdd(host.FullName + "_" + name, s => new Cached<T>(defaultLifeTime, true));
        }

        public ICollection<ICached> Caches => _cache.Values;
    }
}
