import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { VitePWA } from 'vite-plugin-pwa'

// https://vite.dev/config/
export default defineConfig({
  plugins: [
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
            urlPattern: /^https?:\/\/.*\/api\/v3\/mediacover\/track\/.*\/poster\.jpg$/,
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
