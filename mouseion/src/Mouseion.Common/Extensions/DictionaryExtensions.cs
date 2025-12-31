// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Generic;
using System.Linq;

namespace Mouseion.Common.Extensions
{
    public static class DictionaryExtensions
    {
        public static Dictionary<T1, T2> Merge<T1, T2>(this Dictionary<T1, T2> first, Dictionary<T1, T2> second)
        {
            if (first == null)
            {
                throw new ArgumentNullException("first");
            }

            if (second == null)
            {
                throw new ArgumentNullException("second");
            }

            var merged = new Dictionary<T1, T2>();
            first.ToList().ForEach(kv => merged[kv.Key] = kv.Value);
            second.ToList().ForEach(kv => merged[kv.Key] = kv.Value);

            return merged;
        }

        public static void Add<TKey, TValue>(this ICollection<KeyValuePair<TKey, TValue>> collection, TKey key, TValue value)
        {
            collection.Add(new KeyValuePair<TKey, TValue>(key, value));
        }

        public static IDictionary<TNewKey, TNewValue> SelectDictionary<TKey, TValue, TNewKey, TNewValue>(this IDictionary<TKey, TValue> dictionary,
            Func<KeyValuePair<TKey, TValue>, ValueTuple<TNewKey, TNewValue>> selection)
        {
            return dictionary.Select(selection).ToDictionary(t => t.Item1, t => t.Item2);
        }

        public static IDictionary<TNewKey, TNewValue> SelectDictionary<TKey, TValue, TNewKey, TNewValue>(
            this IDictionary<TKey, TValue> dictionary,
            Func<KeyValuePair<TKey, TValue>, TNewKey> keySelector,
            Func<KeyValuePair<TKey, TValue>, TNewValue> valueSelector)
        {
            return dictionary.SelectDictionary(p => { return (keySelector(p), valueSelector(p)); });
        }
    }
}
