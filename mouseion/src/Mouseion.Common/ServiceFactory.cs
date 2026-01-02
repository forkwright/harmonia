// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Generic;
using System.Linq;
using Microsoft.Extensions.DependencyInjection;

namespace Mouseion.Common
{
    public interface IServiceFactory
    {
        T Build<T>()
            where T : class;
        IEnumerable<T> BuildAll<T>()
            where T : class;
        object Build(Type contract);
        IEnumerable<Type> GetImplementations(Type contract);
    }

    public class ServiceFactory : IServiceFactory
    {
        private readonly System.IServiceProvider _container;

        public ServiceFactory(System.IServiceProvider container)
        {
            _container = container;
        }

        public T Build<T>()
            where T : class
        {
            return _container.GetRequiredService<T>();
        }

        public IEnumerable<T> BuildAll<T>()
            where T : class
        {
            return _container.GetServices<T>().GroupBy(c => c.GetType().FullName).Select(g => g.First());
        }

        public object Build(Type contract)
        {
            return _container.GetRequiredService(contract);
        }

        public IEnumerable<Type> GetImplementations(Type contract)
        {
            return _container.GetServices(contract).Select(x => x!.GetType());
        }
    }
}
