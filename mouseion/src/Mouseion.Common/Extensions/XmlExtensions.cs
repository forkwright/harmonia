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
using System.Xml.Linq;

namespace Mouseion.Common.Extensions
{
    public static class XmlExtensions
    {
        public static IEnumerable<XElement> FindDecendants(this XContainer container, string localName)
        {
            return container.Descendants().Where(c => c.Name.LocalName.Equals(localName, StringComparison.InvariantCultureIgnoreCase));
        }

        public static bool TryGetAttributeValue(this XElement element, string name, out string value)
        {
            var attr = element.Attribute(name);

            if (attr != null)
            {
                value = attr.Value;
                return true;
            }
            else
            {
                value = null;
                return false;
            }
        }
    }
}
