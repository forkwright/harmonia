import { defineConfig } from 'vite'
import type { Plugin } from 'vite'
import react from '@vitejs/plugin-react'
import { VitePWA } from 'vite-plugin-pwa'
import fs from 'fs'
import path from 'path'

const LOG_FILE = path.resolve(__dirname, 'error.log')

function devErrorLogger(): Plugin {
  return {
    name: 'dev-error-logger',
    configureServer(server) {
      server.middlewares.use('/api/v3/__dev/log', (req, res) => {
        if (req.method !== 'POST') {
          res.statusCode = 405
          res.end()
          return
        }
        let body = ''
        req.on('data', (chunk: Buffer) => { body += chunk.toString() })
        req.on('end', () => {
          try {
            const entries = JSON.parse(body)
            const lines = entries.map((e: Record<string, string>) =>
              `[${e.timestamp}] ${e.level.toUpperCase().padEnd(5)} [${e.source}] ${e.message}${e.detail ? ' — ' + e.detail : ''}${e.url ? ' @ ' + e.url : ''}${e.stack ? '\n  ' + e.stack.split('\n').slice(1, 4).join('\n  ') : ''}\n`
            ).join('')
            fs.appendFileSync(LOG_FILE, lines)
            // Also print to terminal
            process.stderr.write(lines)
          } catch {
            // Don't let logging break anything
          }
          res.statusCode = 204
          res.end()
        })
      })
    },
  }
}

// https://vite.dev/config/
export default defineConfig({
  server: {
    proxy: {
      '/api/v3': {
        target: 'http://localhost:8787',
        changeOrigin: true,
        bypass(req) {
          // Don't proxy the dev error logger
          if (req.url?.includes('__dev/log')) return req.url
        },
      },
    },
  },
  plugins: [
    devErrorLogger(),
    react(),
    VitePWA({
      registerType: 'autoUpdate',
      includeAssets: ['favicon.ico', 'apple-touch-icon.png', 'mask-icon.svg'],
      manifest: false, // Use existing public/manifest.json
      workbox: {
        globPatterns: ['**/*.{js,css,html,ico,png,svg,jpg,woff2}'],
        runtimeCaching: [
          {
            // Cover art - cache-first with 30 day TTL
            urlPattern: /^https?:\/\/.*\/api\/v3\/mediacover\/\d+\/poster/,
            handler: 'CacheFirst',
            options: {
              cacheName: 'akroasis-cover-art',
              expiration: {
                maxEntries: 500,
                maxAgeSeconds: 60 * 60 * 24 * 30, // 30 days
              },
              cacheableResponse: {
                statuses: [0, 200],
              },
            },
          },
          {
            // Library data (artists, albums, tracks) - stale-while-revalidate
            urlPattern: /^https?:\/\/.*\/api\/v3\/(artists|albums|tracks)/,
            handler: 'StaleWhileRevalidate',
            options: {
              cacheName: 'akroasis-library-data',
              expiration: {
                maxEntries: 200,
                maxAgeSeconds: 60 * 60 * 24, // 24 hours
              },
              cacheableResponse: {
                statuses: [0, 200],
              },
            },
          },
          {
            // Audio streams - network-first with offline fallback
            urlPattern: /^https?:\/\/.*\/api\/v3\/stream\//,
            handler: 'NetworkFirst',
            options: {
              cacheName: 'akroasis-audio-streams',
              expiration: {
                maxEntries: 50,
                maxAgeSeconds: 60 * 60 * 24 * 7, // 7 days
              },
              networkTimeoutSeconds: 10,
              cacheableResponse: {
                statuses: [0, 200],
              },
            },
          },
        ],
        navigateFallback: '/index.html',
        navigateFallbackDenylist: [/^\/api/],
      },
      devOptions: {
        enabled: true, // Enable SW in dev mode for testing
      },
    }),
  ],
})
