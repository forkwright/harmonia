# Akroasis Web/Desktop

React-based frontend for Akroasis, running as:
- Web application (PWA)
- Desktop application (Tauri)

## Requirements

- Node.js 20+
- Rust 1.70+ (for Tauri desktop builds)

## Development

### Web Only
```bash
npm run dev
```

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
- **Desktop**: Tauri 2 (Rust backend)
- **Audio**: Shared akroasis-core Rust crate (via FFI)
- **State**: TBD (Zustand or Jotai)
- **Styling**: Tailwind CSS

## Project Structure

```
web/
├── src/
│   ├── App.tsx
│   ├── main.tsx
│   └── components/
├── src-tauri/
│   ├── src/main.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── package.json
└── vite.config.ts
```

## Phase 0 Complete

Tauri project initialized. Next steps:
1. Install Tailwind CSS
2. Implement design system (bronze/copper colors)
3. Create component library (Phase 3)
