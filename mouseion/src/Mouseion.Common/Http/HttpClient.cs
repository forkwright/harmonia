// Copyright (c) 2025 Mouseion Project
// SPDX-License-Identifier: GPL-3.0-or-later

// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Threading.Tasks;
using Mouseion.Common.Cache;
using Mouseion.Common.EnvironmentInfo;
using Mouseion.Common.Extensions;
using Mouseion.Common.Http.Dispatchers;
using Mouseion.Common.TPL;
using Serilog;

namespace Mouseion.Common.Http
{
    public interface IHttpClient
    {
        Task<HttpResponse> ExecuteAsync(HttpRequest request);
        Task DownloadFileAsync(string url, string fileName);
        Task<HttpResponse> GetAsync(HttpRequest request);
        Task<HttpResponse<T>> GetAsync<T>(HttpRequest request)
            where T : class, new();
        Task<HttpResponse> HeadAsync(HttpRequest request);
        Task<HttpResponse> PostAsync(HttpRequest request);
        Task<HttpResponse<T>> PostAsync<T>(HttpRequest request)
            where T : class, new();
    }

    public class HttpClient : IHttpClient
    {
        private const int MaxRedirects = 5;

        private readonly ILogger _logger;
        private readonly IRateLimitService _rateLimitService;
        private readonly ICached<CookieContainer> _cookieContainerCache;
        private readonly List<IHttpRequestInterceptor> _requestInterceptors;
        private readonly IHttpDispatcher _httpDispatcher;

        public HttpClient(IEnumerable<IHttpRequestInterceptor> requestInterceptors,
            ICacheManager cacheManager,
            IRateLimitService rateLimitService,
            IHttpDispatcher httpDispatcher,
            ILogger logger)
        {
            _requestInterceptors = requestInterceptors.ToList();
            _rateLimitService = rateLimitService;
            _httpDispatcher = httpDispatcher;
            _logger = logger;

            ServicePointManager.DefaultConnectionLimit = 12;
            _cookieContainerCache = cacheManager.GetCache<CookieContainer>(typeof(HttpClient));
        }

        public virtual async Task<HttpResponse> ExecuteAsync(HttpRequest request)
        {
            var cookieContainer = InitializeRequestCookies(request);

            var response = await ExecuteRequestAsync(request, cookieContainer);

            if (request.AllowAutoRedirect && response.HasHttpRedirect)
            {
                var autoRedirectChain = new List<string> { request.Url.ToString() };

                do
                {
                    request.Url += new HttpUri(response.Headers.GetSingleValue("Location"));
                    autoRedirectChain.Add(request.Url.ToString());

                    _logger.Verbose("Redirected to {Url}", request.Url);

                    if (autoRedirectChain.Count > MaxRedirects)
                    {
                        throw new WebException($"Too many automatic redirections were attempted for {autoRedirectChain.Join(" -> ")}", WebExceptionStatus.ProtocolError);
                    }

                    if (RequestRequiresForceGet(response.StatusCode, response.Request.Method))
                    {
                        request.Method = HttpMethod.Get;
                        request.ContentData = null;
                        request.ContentSummary = null;
                    }

                    response = await ExecuteRequestAsync(request, cookieContainer);
                }
                while (response.HasHttpRedirect);
            }

            if (response.HasHttpRedirect && !RuntimeInfo.IsProduction)
            {
                _logger.Error("Server requested a redirect to [{Location}] while in developer mode. Update the request URL to avoid this redirect.", response.Headers["Location"]);
            }

            if (!request.SuppressHttpError && response.HasHttpError && (request.SuppressHttpErrorStatusCodes == null || !request.SuppressHttpErrorStatusCodes.Contains(response.StatusCode)))
            {
                if (request.LogHttpError)
                {
                    _logger.Warning("HTTP Error - {Response}", response);
                }

                if ((int)response.StatusCode == 429)
                {
                    throw new TooManyRequestsException(request, response);
                }
                else
                {
                    throw new HttpException(request, response);
                }
            }

            return response;
        }

        private static bool RequestRequiresForceGet(HttpStatusCode statusCode, HttpMethod requestMethod)
        {
            return statusCode switch
            {
                HttpStatusCode.Moved or HttpStatusCode.Found or HttpStatusCode.MultipleChoices => requestMethod == HttpMethod.Post,
                HttpStatusCode.SeeOther => requestMethod != HttpMethod.Get && requestMethod != HttpMethod.Head,
                _ => false,
            };
        }

        private async Task<HttpResponse> ExecuteRequestAsync(HttpRequest request, CookieContainer cookieContainer)
        {
            foreach (var interceptor in _requestInterceptors)
            {
                request = interceptor.PreRequest(request);
            }

            if (request.RateLimit != TimeSpan.Zero)
            {
                await _rateLimitService.WaitAndPulseAsync(request.Url.Host, request.RateLimitKey, request.RateLimit);
            }

            _logger.Verbose("{@Request}", request);

            var stopWatch = Stopwatch.StartNew();

            var response = await _httpDispatcher.GetResponseAsync(request, cookieContainer);

            HandleResponseCookies(response, cookieContainer);

            stopWatch.Stop();

            _logger.Verbose("{@Response} ({ElapsedMs} ms)", response, stopWatch.ElapsedMilliseconds);

            foreach (var interceptor in _requestInterceptors)
            {
                response = interceptor.PostResponse(response);
            }

            if (request.LogResponseContent)
            {
                _logger.Verbose("Response content ({ContentLength} bytes): {Content}", response.ResponseData.Length, response.Content);
            }

            return response;
        }

        private CookieContainer InitializeRequestCookies(HttpRequest request)
        {
            lock (_cookieContainerCache)
            {
                var sourceContainer = new CookieContainer();

                var presistentContainer = _cookieContainerCache.Get("container", () => new CookieContainer());
                var persistentCookies = presistentContainer.GetCookies((Uri)request.Url);
                sourceContainer.Add(persistentCookies);

                if (request.Cookies.Count != 0)
                {
                    foreach (var pair in request.Cookies)
                    {
                        Cookie cookie;
                        if (pair.Value == null)
                        {
                            cookie = new Cookie(pair.Key, "", "/")
                            {
                                Expires = DateTime.Now.AddDays(-1)
                            };
                        }
                        else
                        {
                            cookie = new Cookie(pair.Key, pair.Value, "/")
                            {
                                Expires = DateTime.Now.AddHours(1)
                            };
                        }

                        sourceContainer.Add((Uri)request.Url, cookie);

                        if (request.StoreRequestCookie)
                        {
                            presistentContainer.Add((Uri)request.Url, cookie);
                        }
                    }
                }

                return sourceContainer;
            }
        }

        private void HandleResponseCookies(HttpResponse response, CookieContainer container)
        {
            foreach (Cookie cookie in container.GetAllCookies())
            {
                cookie.Expired = true;
            }

            var cookieHeaders = response.GetCookieHeaders();

            if (cookieHeaders.Empty())
            {
                return;
            }

            AddCookiesToContainer(response.Request.Url, cookieHeaders, container);

            if (response.Request.StoreResponseCookie)
            {
                lock (_cookieContainerCache)
                {
                    var persistentCookieContainer = _cookieContainerCache.Get("container", () => new CookieContainer());

                    AddCookiesToContainer(response.Request.Url, cookieHeaders, persistentCookieContainer);
                }
            }
        }

        private void AddCookiesToContainer(HttpUri url, string[] cookieHeaders, CookieContainer container)
        {
            foreach (var cookieHeader in cookieHeaders)
            {
                try
                {
                    container.SetCookies((Uri)url, cookieHeader);
                }
                catch (System.Net.CookieException ex)
                {
                    _logger.Debug(ex, "Invalid cookie format in {Url}", url);
                }
                catch (ArgumentException ex)
                {
                    _logger.Debug(ex, "Invalid cookie argument in {Url}", url);
                }
            }
        }

        public async Task DownloadFileAsync(string url, string fileName)
        {
            var fileNamePart = fileName + ".part";

            try
            {
                var fileInfo = new FileInfo(fileName);
                if (fileInfo.Directory != null && !fileInfo.Directory.Exists)
                {
                    fileInfo.Directory.Create();
                }

                _logger.Debug("Downloading [{Url}] to [{FileName}]", url.SanitizeForLog(), fileName.SanitizeForLog());

                var stopWatch = Stopwatch.StartNew();
                await using (var fileStream = new FileStream(fileNamePart, FileMode.Create, FileAccess.ReadWrite))
                {
                    var request = new HttpRequest(url);
                    request.AllowAutoRedirect = true;
                    request.ResponseStream = fileStream;
                    request.RequestTimeout = TimeSpan.FromSeconds(300);
                    var response = await GetAsync(request);

                    if (response.Headers.ContentType != null && response.Headers.ContentType.Contains("text/html"))
                    {
                        throw new HttpException(request, response, "Site responded with html content.");
                    }
                }

                stopWatch.Stop();

                if (File.Exists(fileName))
                {
                    File.Delete(fileName);
                }

                File.Move(fileNamePart, fileName);
                _logger.Debug("Downloading Completed. took {ElapsedSeconds:0}s", stopWatch.Elapsed.Seconds);
            }
            finally
            {
                if (File.Exists(fileNamePart))
                {
                    File.Delete(fileNamePart);
                }
            }
        }

        public Task<HttpResponse> GetAsync(HttpRequest request)
        {
            request.Method = HttpMethod.Get;
            return ExecuteAsync(request);
        }

        public async Task<HttpResponse<T>> GetAsync<T>(HttpRequest request)
            where T : class, new()
        {
            var response = await GetAsync(request);
            CheckResponseContentType(response);
            return new HttpResponse<T>(response);
        }

        public Task<HttpResponse> HeadAsync(HttpRequest request)
        {
            request.Method = HttpMethod.Head;
            return ExecuteAsync(request);
        }

        public Task<HttpResponse> PostAsync(HttpRequest request)
        {
            request.Method = HttpMethod.Post;
            return ExecuteAsync(request);
        }

        public async Task<HttpResponse<T>> PostAsync<T>(HttpRequest request)
            where T : class, new()
        {
            var response = await PostAsync(request);
            CheckResponseContentType(response);
            return new HttpResponse<T>(response);
        }

        private static void CheckResponseContentType(HttpResponse response)
        {
            if (response.Headers.ContentType != null && response.Headers.ContentType.Contains("text/html"))
            {
                throw new UnexpectedHtmlContentException(response);
            }
        }
    }
}
