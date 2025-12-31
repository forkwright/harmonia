// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;
using Mouseion.Common.EnsureThat;

namespace Mouseion.Common.Cache
{
    public class Cached<T> : ICached<T>
    {
        private class CacheItem
        {
            public T Object { get; private set; }
            public DateTime? ExpiryTime { get; private set; }

            public CacheItem(T obj, TimeSpan? lifetime = null)
            {
                Object = obj;
                if (lifetime.HasValue)
                {
                    ExpiryTime = DateTime.UtcNow + lifetime.Value;
                }
            }

            public bool IsExpired()
            {
                return ExpiryTime.HasValue && ExpiryTime.Value < DateTime.UtcNow;
            }
        }

        private readonly ConcurrentDictionary<string, CacheItem> _store;

        private readonly TimeSpan? _defaultLifeTime;
        private readonly bool _rollingExpiry;

        public Cached(TimeSpan? defaultLifeTime = null, bool rollingExpiry = false)
        {
            _store = new ConcurrentDictionary<string, CacheItem>();
            _defaultLifeTime = defaultLifeTime;
            _rollingExpiry = rollingExpiry;
        }

        public void Set(string key, T value, TimeSpan? lifetime = null)
        {
            Ensure.That(key, () => key).IsNotNullOrWhiteSpace();
            lifetime = lifetime ?? _defaultLifeTime;
            _store[key] = new CacheItem(value, lifetime);
        }

        public T Get(string key, Func<T> function, TimeSpan? lifeTime = null)
        {
            Ensure.That(key, () => key).IsNotNullOrWhiteSpace();

            lifeTime = lifeTime ?? _defaultLifeTime;

            CacheItem cacheItem;
            T value;

            if (!_store.TryGetValue(key, out cacheItem) || cacheItem.IsExpired())
            {
                value = function();
                Set(key, value, lifeTime);
            }
            else
            {
                if (_rollingExpiry && lifeTime.HasValue)
                {
                    _store.TryUpdate(key, new CacheItem(cacheItem.Object, lifeTime), cacheItem);
                }

                value = cacheItem.Object;
            }

            return value;
        }

        public T Find(string key)
        {
            CacheItem cacheItem;
            if (_store.TryGetValue(key, out cacheItem) && !cacheItem.IsExpired())
            {
                if (_rollingExpiry && _defaultLifeTime.HasValue)
                {
                    _store.TryUpdate(key, new CacheItem(cacheItem.Object, _defaultLifeTime), cacheItem);
                }

                return cacheItem.Object;
            }

            return default!;
        }

        public void Remove(string key)
        {
            CacheItem cacheItem;
            _store.TryRemove(key, out cacheItem);
        }

        public void Clear()
        {
            _store.Clear();
        }

        public void ClearExpired()
        {
            foreach (var cached in _store.Where(c => c.Value.IsExpired()))
            {
                Remove(cached.Key);
            }
        }

        public bool ContainsKey(string key)
        {
            return _store.ContainsKey(key);
        }

        public ICollection<T> Values
        {
            get
            {
                return _store.Values.Select(c => c.Object).ToList();
            }
        }

        public ICollection<string> Keys
        {
            get
            {
                return _store.Keys.ToList();
            }
        }
    }
}
