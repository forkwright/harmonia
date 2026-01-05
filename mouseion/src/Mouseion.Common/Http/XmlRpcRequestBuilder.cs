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
using System.Xml.Linq;
using Serilog;

namespace Mouseion.Common.Http
{
    public class XmlRpcRequestBuilder : HttpRequestBuilder
    {
        public static string XmlRpcContentType = "text/xml";

        private readonly ILogger _logger;

        public string XmlMethod { get; private set; }
        public List<object> XmlParameters { get; private set; }

        public XmlRpcRequestBuilder(string baseUrl, ILogger logger)
            : base(baseUrl)
        {
            Method = HttpMethod.Post;
            XmlParameters = new List<object>();
            _logger = logger;
        }

        public XmlRpcRequestBuilder(bool useHttps, string host, int port, ILogger logger, string urlBase = null)
            : this(BuildBaseUrl(useHttps, host, port, urlBase), logger)
        {
        }

        public override HttpRequestBuilder Clone()
        {
            if (base.Clone() is not XmlRpcRequestBuilder clone)
            {
                throw new InvalidOperationException("Clone must return XmlRpcRequestBuilder");
            }

            clone.XmlParameters = new List<object>(XmlParameters);
            return clone;
        }

        public XmlRpcRequestBuilder Call(string method, params object[] parameters)
        {
            if (Clone() is not XmlRpcRequestBuilder clone)
            {
                throw new InvalidOperationException("Clone must return XmlRpcRequestBuilder");
            }

            clone.XmlMethod = method;
            clone.XmlParameters = parameters.ToList();
            return clone;
        }

        protected override void Apply(HttpRequest request)
        {
            base.Apply(request);

            request.Headers.ContentType = XmlRpcContentType;

            var methodCallElements = new List<XElement> { new XElement("methodName", XmlMethod) };

            if (XmlParameters.Any())
            {
                var argElements = XmlParameters.Select(x => new XElement("param", ConvertParameter(x))).ToList();
                var paramsElement = new XElement("params", argElements);
                methodCallElements.Add(paramsElement);
            }

            var message = new XDocument(
                new XDeclaration("1.0", "utf-8", "yes"),
                new XElement("methodCall", methodCallElements));

            var body = message.ToString();

            _logger.Debug("Executing remote method: {XmlMethod}", XmlMethod);

            _logger.Verbose("methodCall {XmlMethod} body:\n{Body}", XmlMethod, body);

            request.SetContent(body);
        }

        private static XElement ConvertParameter(object value)
        {
            XElement data;

            if (value is string s)
            {
                data = new XElement("string", s);
            }
            else if (value is List<string> l)
            {
                data = new XElement("array", new XElement("data", l.Select(x => new XElement("value", new XElement("string", x)))));
            }
            else if (value is int i)
            {
                data = new XElement("int", i);
            }
            else if (value is byte[] bytes)
            {
                data = new XElement("base64", Convert.ToBase64String(bytes));
            }
            else if (value is Dictionary<string, string> d)
            {
                data = new XElement("struct", d.Select(p => new XElement("member", new XElement("name", p.Key), new XElement("value", p.Value))));
            }
            else
            {
                throw new InvalidOperationException($"Unhandled argument type {value.GetType().Name}");
            }

            return new XElement("value", data);
        }
    }
}
