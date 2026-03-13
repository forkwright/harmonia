# Spec 09: UI design system & theme architecture

**Status:** Draft
**Priority:** High
**Depends On:** None (foundation for all UI work)

## Goal

Establish a principled design system for Akroasis that supports light and dark themes, defines component behavior, and creates the visual language the entire app speaks. Not a component library: a set of decisions that make future decisions unnecessary. Every element should be intentional: present because it serves the user, absent because it doesn't.

## Design principles

1. **The content is the interface.** Album art, waveforms, track lists: these ARE the app. Chrome exists to frame them, not compete with them.
2. **Information density without clutter.** Show the signal path, the format, the quality, but only when the user is in context to care. Progressive disclosure, not hidden options.
3. **Tactile, not flat.** Subtle depth through shadows, blur, and border opacity. The difference between a surface you want to touch and a spreadsheet.
4. **Consistent motion.** 150ms for micro-interactions (hover, active states). 300ms for layout transitions (expand, collapse, navigate). No motion for motion's sake.
5. **Every state is designed.** Empty, loading, error, partial, complete. If a state can exist, it has a design.

## Phases

### Phase 1: CSS custom property architecture

Replace hardcoded Tailwind color classes with CSS custom properties. Same approach as Aletheia Spec 29 Phase 4.

```css
:root {
  /* Surface hierarchy */
  --surface-base: #0a0a0a;         /* App background */
  --surface-raised: #1a1412;       /* Cards, panels */
  --surface-overlay: #231c17;      /* Dropdowns, modals */
  --surface-sunken: #050505;       /* Inset areas, wells */

  /* Content hierarchy */
  --text-primary: #f0e6da;         /* Titles, important */
  --text-secondary: #c0a586;       /* Body, descriptions */
  --text-tertiary: #7a6553;        /* Metadata, timestamps */
  --text-muted: #4a3d33;           /* Disabled, placeholder */

  /* Accent */
  --accent-primary: #b08968;       /* Interactive elements, active states */
  --accent-hover: #c9a07a;         /* Hover states */
  --accent-active: #967252;        /* Pressed states */

  /* Borders */
  --border-subtle: rgba(176, 137, 104, 0.1);
  --border-default: rgba(176, 137, 104, 0.2);
  --border-strong: rgba(176, 137, 104, 0.35);

  /* Signal quality (Roon-inspired) */
  --quality-enhanced: #a78bfa;     /* Purple — hi-res, enhanced */
  --quality-lossless: #60a5fa;     /* Blue — bit-perfect lossless */
  --quality-high: #34d399;         /* Green — high quality */
  --quality-standard: #fbbf24;     /* Amber — lossy or resampled */
  --quality-low: #f87171;          /* Red — significant degradation */

  /* Semantic */
  --error-bg: rgba(153, 27, 27, 0.3);
  --error-border: rgba(185, 28, 28, 0.5);
  --error-text: #fca5a5;
  --success-bg: rgba(6, 78, 59, 0.3);
  --success-text: #6ee7b7;
}

[data-theme="light"] {
  --surface-base: #f7f3e8;         /* Warm parchment */
  --surface-raised: #ffffff;
  --surface-overlay: #ffffff;
  --surface-sunken: #ede8db;

  --text-primary: #1a1412;
  --text-secondary: #5f4b3c;
  --text-tertiary: #8c7a6a;
  --text-muted: #b8a898;

  --accent-primary: #8b6a4f;
  --accent-hover: #7a5a40;
  --accent-active: #6b4c34;

  --border-subtle: rgba(139, 106, 79, 0.08);
  --border-default: rgba(139, 106, 79, 0.15);
  --border-strong: rgba(139, 106, 79, 0.25);

  /* Signal quality colors work on both themes */
  --quality-enhanced: #7c3aed;
  --quality-lossless: #2563eb;
  --quality-high: #059669;
  --quality-standard: #d97706;
  --quality-low: #dc2626;

  --error-bg: rgba(254, 226, 226, 0.8);
  --error-border: rgba(248, 113, 113, 0.5);
  --error-text: #991b1b;
  --success-bg: rgba(209, 250, 229, 0.8);
  --success-text: #065f46;
}
```

- [ ] Define CSS custom property schema covering all color roles
- [ ] Add `[data-theme="light"]` override block to `index.css`
- [ ] Add inline `<script>` in `index.html` for FOUC prevention (read localStorage before paint)
- [ ] Migrate existing hardcoded Tailwind classes to use CSS variables via `theme.extend.colors` in tailwind config
- [ ] Settings page: theme toggle (system / light / dark)
- [ ] Persist preference to `localStorage`

### Phase 2: component refinement

Not a component library overhaul; targeted improvements to existing primitives.

- [ ] **Card**: support `variant` prop: `raised` (default), `flat`, `inset`. Hover state standardized.
- [ ] **Button**: add `ghost` variant (no background, text-only). Add `icon` variant (square, icon-only).
- [ ] **Input**: focus ring using `--accent-primary`. Label as `--text-tertiary`.
- [ ] **Badge**: reusable for format labels, quality indicators. Semantic color variants.
- [ ] **Skeleton**: extract from LibraryPage into shared component. Standard pulse animation.
- [ ] **EmptyState**: extract from LibraryPage into shared component. Icon + title + subtitle.
- [ ] **Tooltip**: for truncated text, icon-only buttons, signal path nodes. 150ms delay, 200ms fade.

### Phase 3: motion & interaction

- [ ] Define transition timing: `--duration-fast: 100ms`, `--duration-normal: 150ms`, `--duration-slow: 300ms`
- [ ] Page transitions: fade + slight vertical shift (prevent jarring route changes)
- [ ] List item stagger on initial load (50ms per item, max 10 items)
- [ ] Press feedback on buttons (scale 0.97 on active)
- [ ] Range input: thumb grows slightly on hover
- [ ] Scroll-linked nav shadow: nav bar gains `shadow-lg` after scrolling past threshold

### Phase 4: typography & spacing scale

- [ ] Define type scale: display / heading / subheading / body / caption / overline
- [ ] Standardize spacing: 4px base unit, consistent gaps (4, 8, 12, 16, 24, 32, 48)
- [ ] Monospace for all numeric data (durations, bitrates, percentages): `tabular-nums` applied globally via utility class
- [ ] Ensure bronze color ramp has sufficient contrast ratios (WCAG AA: 4.5:1 for body, 3:1 for large text) on both themes

## Dependencies

- None; this is pure frontend, no backend changes

## Notes

- Light theme should feel warm and intentional, not "invert the colors." The Aletheia approach (parchment base, `#F7F3E8`) worked well.
- CSS custom properties over Tailwind `dark:` classes because: (a) custom property approach works with any JS framework, (b) allows runtime theme switching without class gymnastics, (c) cleaner than duplicating every color class.
- Motion budget: never more than 300ms. Users should feel the UI is responsive, not animated.
- Skeleton loading is more honest than spinners; it tells the user what shape the content will take.
- The signal quality colors (Phase 1) are defined here but used in Spec 12 (Signal Path). Defining them centrally prevents drift.
