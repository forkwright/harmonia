// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Generic;
using System.Linq;

namespace Mouseion.Common.Composition
{
    public class KnownTypes
    {
        private List<Type> _knownTypes;

        public KnownTypes()
            : this(new List<Type>())
        {
        }

        public KnownTypes(List<Type> loadedTypes)
        {
            _knownTypes = loadedTypes;
        }

        public IEnumerable<Type> GetImplementations(Type contractType)
        {
            return _knownTypes
                .Where(implementation =>
                    contractType.IsAssignableFrom(implementation) &&
                    !implementation.IsInterface &&
                    !implementation.IsAbstract);
        }
    }
}
