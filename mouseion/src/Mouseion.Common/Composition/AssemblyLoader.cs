// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.Loader;
using Mouseion.Common.EnvironmentInfo;

namespace Mouseion.Common.Composition
{
    public class AssemblyLoader
    {
        static AssemblyLoader()
        {
            AppDomain.CurrentDomain.AssemblyResolve += new ResolveEventHandler(ContainerResolveEventHandler);
        }

        public static IList<Assembly> Load(IList<string> assemblyNames)
        {
            var toLoad = assemblyNames.ToList();
            toLoad.Add("Mouseion.Common");
            toLoad.Add(OsInfo.IsWindows ? "Mouseion.Windows" : "Mouseion.Mono");

            var toRegisterResolver = new List<string> { "System.Data.SQLite" };
            toRegisterResolver.AddRange(assemblyNames.Intersect(new[] { "Mouseion.Core" }));
            RegisterNativeResolver(toRegisterResolver);

            var startupPath = AppDomain.CurrentDomain.BaseDirectory;

            return toLoad
                .Select(x => AssemblyLoadContext.Default.LoadFromAssemblyPath(Path.Combine(startupPath, $"{x}.dll")))
                .ToList();
        }

        private static Assembly? ContainerResolveEventHandler(object? sender, ResolveEventArgs args)
        {
            if (args.RequestingAssembly?.Location == null)
            {
                return null;
            }

            var resolver = new AssemblyDependencyResolver(args.RequestingAssembly.Location);
            var assemblyPath = resolver.ResolveAssemblyToPath(new AssemblyName(args.Name!));

            if (assemblyPath == null)
            {
                return null;
            }

            return AssemblyLoadContext.Default.LoadFromAssemblyPath(assemblyPath);
        }

        public static void RegisterNativeResolver(IEnumerable<string> assemblyNames)
        {
            foreach (var name in assemblyNames)
            {
                var assemblyPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, $"{name}.dll");
                if (!File.Exists(assemblyPath))
                {
                    continue;
                }

                var assembly = AssemblyLoadContext.Default.LoadFromAssemblyPath(assemblyPath);

                try
                {
                    NativeLibrary.SetDllImportResolver(assembly, LoadNativeLib);
                }
                catch (InvalidOperationException)
                {
                    // This can only be set once per assembly
                }
            }
        }

        private static IntPtr LoadNativeLib(string libraryName, Assembly assembly, DllImportSearchPath? dllImportSearchPath)
        {
            var mappedName = libraryName;
            if (OsInfo.IsLinux)
            {
                if (libraryName == "sqlite3")
                {
                    mappedName = "libsqlite3.so.0";
                }
            }

            return NativeLibrary.Load(mappedName, assembly, dllImportSearchPath);
        }
    }
}
