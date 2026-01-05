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
using System.Net.Http;
using Mouseion.Common.Serializer;
using Newtonsoft.Json;

namespace Mouseion.Common.Http
{
    public class JsonRpcRequestBuilder : HttpRequestBuilder
    {
        public static HttpAccept JsonRpcHttpAccept = new HttpAccept("application/json-rpc, application/json");
        public static string JsonRpcContentType = "application/json";

        public string JsonMethod { get; private set; }
        public List<object> JsonParameters { get; private set; }

        public JsonRpcRequestBuilder(string baseUrl)
            : base(baseUrl)
        {
            Method = HttpMethod.Post;
            JsonParameters = new List<object>();
        }

        public JsonRpcRequestBuilder(string baseUrl, string method, IEnumerable<object> parameters)
            : base(baseUrl)
        {
            Method = HttpMethod.Post;
            JsonMethod = method;
            JsonParameters = parameters.ToList();
        }

        public override HttpRequestBuilder Clone()
        {
            if (base.Clone() is not JsonRpcRequestBuilder clone)
            {
                throw new InvalidOperationException("Clone must return JsonRpcRequestBuilder");
            }

            clone.JsonParameters = new List<object>(JsonParameters);
            return clone;
        }

        public JsonRpcRequestBuilder Call(string method, params object[] parameters)
        {
            if (Clone() is not JsonRpcRequestBuilder clone)
            {
                throw new InvalidOperationException("Clone must return JsonRpcRequestBuilder");
            }

            clone.JsonMethod = method;
            clone.JsonParameters = parameters.ToList();
            return clone;
        }

        protected override void Apply(HttpRequest request)
        {
            base.Apply(request);

            request.Headers.ContentType = JsonRpcContentType;

            var parameterData = new object[JsonParameters.Count];
            var parameterSummary = new string[JsonParameters.Count];

            for (var i = 0; i < JsonParameters.Count; i++)
            {
                ConvertParameter(JsonParameters[i], out parameterData[i], out parameterSummary[i]);
            }

            var message = new Dictionary<string, object>();
            message["jsonrpc"] = "2.0";
            message["method"] = JsonMethod;
            message["params"] = parameterData;
            message["id"] = CreateNextId();

            request.SetContent(Json.ToJson(message));

            if (request.ContentSummary == null)
            {
                request.ContentSummary = string.Format("{0}({1})", JsonMethod, string.Join(", ", parameterSummary));
            }
        }

        private static void ConvertParameter(object value, out object data, out string summary)
        {
            if (value is byte[] bytes)
            {
                data = Convert.ToBase64String(bytes);
                summary = string.Format("[blob {0} bytes]", bytes.Length);
            }
            else if (value is Array && ((Array)value).Length > 0)
            {
                data = value;
                summary = "[...]";
            }
            else
            {
                data = value;
                summary = JsonConvert.SerializeObject(data);
            }
        }

        public static string CreateNextId()
        {
            return Guid.NewGuid().ToString().Substring(0, 8);
        }
    }
}
