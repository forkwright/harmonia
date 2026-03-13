# Development guide

## Mock API server

**How It Works:**
- **Mock Service Worker (MSW)**: Intercepts API requests
- **Dev-only**: Auto-enabled in `npm run dev`
- **Realistic data**: Mock tracks, albums, artists

### Running locally

```bash
npm run dev
```

The app will:
1. Start on `http://localhost:5173`
2. Automatically enable MSW
3. Prefill login form with mock credentials
4. Intercept all API calls to `http://localhost:5000`

### Mock credentials

**Development mode auto-fills**:
- Server URL: `http://localhost:5000`
- Username: `admin`
- Password: `password`

Just click "Login" - all credentials are accepted.

### Mock data

Located in `src/mocks/data.ts`:
- **4 Artists**: Pink Floyd, Radiohead, Miles Davis, J.S. Bach
- **4 Albums**: Dark Side of the Moon, Wish You Were Here, OK Computer, Kind of Blue
- **7 Tracks**: Various tracks with realistic high-res audio metadata (24/96, 24/192 FLAC)

### Customizing mock data

Edit `src/mocks/data.ts` to add more mock content:

```typescript
export const mockTracks: Track[] = [
  {
    id: 8,
    title: 'Your Track',
    artist: 'Your Artist',
    album: 'Your Album',
    duration: 300000,  // 5 minutes in ms
    // ... other fields
  },
]
```

### Testing with real backend

To test with a real Mouseion instance:

1. **Disable mock server**: Comment out MSW initialization in `src/main.tsx`
2. **Set server URL**: Update login form or localStorage
3. **Run Mouseion**: Ensure backend is running on your network

### Mock API endpoints

All Mouseion v3 endpoints are mocked:

- `POST /api/v3/auth/login` - Returns mock token
- `GET /api/v3/artists` - Returns mock artists
- `GET /api/v3/albums` - Returns mock albums
- `GET /api/v3/tracks` - Returns mock tracks
- `GET /api/v3/stream/:id` - Returns mock stream response
- `GET /api/v3/mediacover/track/:id/poster.jpg` - Redirects to placeholder images

### Network tab

Open browser DevTools → Network to see MSW intercepting requests with `[MSW]` prefix.

### Limitations

- **No actual audio streaming**: Mock audio endpoint returns placeholder
- **No persistence**: Refresh resets mock state
- **Fixed data**: Modifications don't persist between reloads

For full integration testing, use a real Mouseion backend.
