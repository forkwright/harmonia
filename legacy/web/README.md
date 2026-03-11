# Akroasis Web/Desktop

React-based frontend for Akroasis, running as:
- Web application (PWA)
- Desktop application (Tauri)

## Requirements

- Node.js 20+
- Rust 1.70+ (for Tauri desktop builds)

## Development

### Web Only (with Mock API)
```bash
npm run dev
```

Opens `http://localhost:5173` with mock API enabled. Login credentials auto-filled.

See [DEVELOPMENT.md](DEVELOPMENT.md) for mock API details.

### Desktop (Tauri)
```bash
npm run tauri:dev
```

## Building

### Web Build
```bash
npm run build
```

### Desktop Build
```bash
npm run tauri:build
```

## Architecture

- **Frontend**: React 19 + TypeScript + Vite
- **Styling**: Tailwind CSS (bronze/copper theme)
- **Routing**: React Router
- **State**: Zustand
- **Desktop**: Tauri 2 (Rust backend)
- **Audio**: HTML5 Audio API (Web Audio API planned)

## Project Structure

```
web/
├── src/
│   ├── api/           # Mouseion API client
│   ├── components/    # UI components
│   ├── pages/         # Page components
│   ├── stores/        # Zustand state stores
│   ├── types/         # TypeScript types
│   ├── App.tsx
│   └── main.tsx
├── src-tauri/         # Tauri desktop backend
├── package.json
└── vite.config.ts
```

## Features Implemented

### Phase 3 Foundation (Current)
- ✅ Tailwind CSS with bronze/copper design system
- ✅ React Router navigation
- ✅ Zustand state management
- ✅ Mouseion API client
- ✅ Login screen
- ✅ Basic Now Playing UI
- ✅ HTML5 Audio playback
- ✅ Mock API server (MSW) for local testing

### Planned
- Library browsing (Artists/Albums/Tracks)
- Queue management
- Keyboard shortcuts
- PWA support
- Service worker offline caching
- Media Session API
- Web Audio API (gapless playback)

## Current Status

**Phase 3**: In Progress
- Foundation complete
- Waiting for Mouseion backend (Week 3+)
- Next: Library browsing UI
