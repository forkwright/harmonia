//! Runtime design tokens for the Harmonia desktop app.
//!
//! The token records mirror the W3C DTCG shape: each token carries a path,
//! a `$value`, and a `$type`. The runtime export is CSS custom properties
//! scoped to the Harmonia app root so Dioxus components can consume `var(...)`
//! directly without a build-time styling pipeline.

use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DtcgType {
    Color,
    CubicBezier,
    Dimension,
    Duration,
    FontFamily,
    FontWeight,
    Number,
    Shadow,
}

impl DtcgType {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Color => "color",
            Self::CubicBezier => "cubicBezier",
            Self::Dimension => "dimension",
            Self::Duration => "duration",
            Self::FontFamily => "fontFamily",
            Self::FontWeight => "fontWeight",
            Self::Number => "number",
            Self::Shadow => "shadow",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DtcgToken {
    pub(crate) path: &'static str,
    pub(crate) name: &'static str,
    pub(crate) value: &'static str,
    pub(crate) token_type: DtcgType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TokenGroup {
    pub(crate) name: &'static str,
    pub(crate) tokens: &'static [DtcgToken],
}

const COLOR_TOKENS: &[DtcgToken] = &[
    token("color.accent", "accent", "#9A7B4F", DtcgType::Color),
    token(
        "color.accent-hover",
        "accent-hover",
        "#B08E5C",
        DtcgType::Color,
    ),
    token("color.accent-dim", "accent-dim", "#7A6340", DtcgType::Color),
    token(
        "color.accent-muted",
        "accent-muted",
        "rgba(154, 123, 79, 0.4)",
        DtcgType::Color,
    ),
    token(
        "color.border-focused",
        "border-focused",
        "var(--accent)",
        DtcgType::Color,
    ),
    token(
        "color.border-selected",
        "border-selected",
        "var(--accent)",
        DtcgType::Color,
    ),
    token(
        "color.input-border-focus",
        "input-border-focus",
        "var(--accent)",
        DtcgType::Color,
    ),
    token(
        "color.focus-ring",
        "focus-ring",
        "rgba(154, 123, 79, 0.35)",
        DtcgType::Color,
    ),
];

const TYPOGRAPHY_TOKENS: &[DtcgToken] = &[
    token(
        "typography.font-mono",
        "font-mono",
        "\"IBM Plex Mono\", ui-monospace, \"JetBrains Mono\", \"Fira Code\", monospace",
        DtcgType::FontFamily,
    ),
    token(
        "typography.font-display",
        "font-display",
        "\"Cormorant Garamond\", \"Garamond\", Georgia, serif",
        DtcgType::FontFamily,
    ),
    token(
        "typography.font-sans",
        "font-sans",
        "system-ui, -apple-system, \"Segoe UI\", \"Helvetica Neue\", sans-serif",
        DtcgType::FontFamily,
    ),
    token(
        "typography.text-xs",
        "text-xs",
        "0.694rem",
        DtcgType::Dimension,
    ),
    token(
        "typography.text-sm",
        "text-sm",
        "0.833rem",
        DtcgType::Dimension,
    ),
    token(
        "typography.text-base",
        "text-base",
        "0.875rem",
        DtcgType::Dimension,
    ),
    token(
        "typography.text-md",
        "text-md",
        "1.05rem",
        DtcgType::Dimension,
    ),
    token(
        "typography.text-lg",
        "text-lg",
        "1.26rem",
        DtcgType::Dimension,
    ),
    token(
        "typography.text-xl",
        "text-xl",
        "1.512rem",
        DtcgType::Dimension,
    ),
    token(
        "typography.text-2xl",
        "text-2xl",
        "1.814rem",
        DtcgType::Dimension,
    ),
    token(
        "typography.text-3xl",
        "text-3xl",
        "2.177rem",
        DtcgType::Dimension,
    ),
    token(
        "typography.leading-tight",
        "leading-tight",
        "1.25",
        DtcgType::Number,
    ),
    token(
        "typography.leading-normal",
        "leading-normal",
        "1.5",
        DtcgType::Number,
    ),
    token(
        "typography.leading-relaxed",
        "leading-relaxed",
        "1.625",
        DtcgType::Number,
    ),
    token(
        "typography.weight-normal",
        "weight-normal",
        "400",
        DtcgType::FontWeight,
    ),
    token(
        "typography.weight-medium",
        "weight-medium",
        "500",
        DtcgType::FontWeight,
    ),
    token(
        "typography.weight-semibold",
        "weight-semibold",
        "600",
        DtcgType::FontWeight,
    ),
    token(
        "typography.weight-bold",
        "weight-bold",
        "700",
        DtcgType::FontWeight,
    ),
    token(
        "typography.tracking-normal",
        "tracking-normal",
        "0",
        DtcgType::Dimension,
    ),
    token(
        "typography.tracking-wide",
        "tracking-wide",
        "0.04em",
        DtcgType::Dimension,
    ),
];

const SPACING_TOKENS: &[DtcgToken] = &[
    token("spacing.space-1", "space-1", "0.25rem", DtcgType::Dimension),
    token("spacing.space-2", "space-2", "0.5rem", DtcgType::Dimension),
    token("spacing.space-3", "space-3", "0.75rem", DtcgType::Dimension),
    token("spacing.space-4", "space-4", "1rem", DtcgType::Dimension),
    token("spacing.space-5", "space-5", "1.25rem", DtcgType::Dimension),
    token("spacing.space-6", "space-6", "1.5rem", DtcgType::Dimension),
    token("spacing.space-8", "space-8", "2rem", DtcgType::Dimension),
    token("spacing.space-12", "space-12", "3rem", DtcgType::Dimension),
    token("spacing.space-16", "space-16", "4rem", DtcgType::Dimension),
];

const RADIUS_TOKENS: &[DtcgToken] = &[
    token(
        "radii.radius-sm",
        "radius-sm",
        "0.125rem",
        DtcgType::Dimension,
    ),
    token(
        "radii.radius-md",
        "radius-md",
        "0.25rem",
        DtcgType::Dimension,
    ),
    token(
        "radii.radius-lg",
        "radius-lg",
        "0.5rem",
        DtcgType::Dimension,
    ),
    token(
        "radii.radius-xl",
        "radius-xl",
        "0.75rem",
        DtcgType::Dimension,
    ),
    token(
        "radii.radius-full",
        "radius-full",
        "9999px",
        DtcgType::Dimension,
    ),
];

const SHADOW_TOKENS: &[DtcgToken] = &[
    token(
        "shadows.shadow-sm",
        "shadow-sm",
        "inset 0 1px 0 rgba(255, 255, 255, 0.04)",
        DtcgType::Shadow,
    ),
    token(
        "shadows.shadow-card",
        "shadow-card",
        "0 1px 2px rgba(0, 0, 0, 0.28)",
        DtcgType::Shadow,
    ),
    token(
        "shadows.shadow-float",
        "shadow-float",
        "0 8px 24px rgba(0, 0, 0, 0.32)",
        DtcgType::Shadow,
    ),
    token(
        "shadows.shadow-modal",
        "shadow-modal",
        "0 18px 56px rgba(0, 0, 0, 0.44)",
        DtcgType::Shadow,
    ),
    token(
        "shadows.shadow-glow",
        "shadow-glow",
        "0 0 0 3px var(--focus-ring)",
        DtcgType::Shadow,
    ),
];

const MOTION_TOKENS: &[DtcgToken] = &[
    token(
        "motion.duration-fast",
        "duration-fast",
        "120ms",
        DtcgType::Duration,
    ),
    token(
        "motion.duration-normal",
        "duration-normal",
        "200ms",
        DtcgType::Duration,
    ),
    token(
        "motion.duration-slow",
        "duration-slow",
        "350ms",
        DtcgType::Duration,
    ),
    token(
        "motion.ease-out-expo",
        "ease-out-expo",
        "cubic-bezier(0.16, 1, 0.3, 1)",
        DtcgType::CubicBezier,
    ),
    token(
        "motion.ease-in-out",
        "ease-in-out",
        "cubic-bezier(0.4, 0, 0.2, 1)",
        DtcgType::CubicBezier,
    ),
];

const Z_INDEX_TOKENS: &[DtcgToken] = &[
    token("z-index.z-content", "z-content", "10", DtcgType::Number),
    token("z-index.z-modal", "z-modal", "100", DtcgType::Number),
    token("z-index.z-tooltip", "z-tooltip", "1000", DtcgType::Number),
    token("z-index.z-toast", "z-toast", "9999", DtcgType::Number),
];

const DARK_THEME_TOKENS: &[DtcgToken] = &[
    token("color.bg", "bg", "#12110f", DtcgType::Color),
    token("color.bg-surface", "bg-surface", "#1b1814", DtcgType::Color),
    token(
        "color.bg-surface-bright",
        "bg-surface-bright",
        "#252018",
        DtcgType::Color,
    ),
    token(
        "color.bg-surface-dim",
        "bg-surface-dim",
        "#0c0b0a",
        DtcgType::Color,
    ),
    token(
        "color.bg-overlay",
        "bg-overlay",
        "rgba(12, 11, 10, 0.72)",
        DtcgType::Color,
    ),
    token(
        "color.bg-hover",
        "bg-hover",
        "var(--bg-surface-bright)",
        DtcgType::Color,
    ),
    token(
        "color.bg-elevated",
        "bg-elevated",
        "var(--bg-surface-bright)",
        DtcgType::Color,
    ),
    token(
        "color.text-primary",
        "text-primary",
        "#f0ece2",
        DtcgType::Color,
    ),
    token(
        "color.text-secondary",
        "text-secondary",
        "#c8bda8",
        DtcgType::Color,
    ),
    token("color.text-muted", "text-muted", "#8f826d", DtcgType::Color),
    token(
        "color.text-inverse",
        "text-inverse",
        "#12110f",
        DtcgType::Color,
    ),
    token("color.border", "border", "#332c22", DtcgType::Color),
    token(
        "color.border-separator",
        "border-separator",
        "#282219",
        DtcgType::Color,
    ),
    token("color.input-bg", "input-bg", "#171510", DtcgType::Color),
    token(
        "color.input-border",
        "input-border",
        "#403728",
        DtcgType::Color,
    ),
    token(
        "color.status-success",
        "status-success",
        "#7AA582",
        DtcgType::Color,
    ),
    token(
        "color.status-success-bg",
        "status-success-bg",
        "#182018",
        DtcgType::Color,
    ),
    token(
        "color.status-warning",
        "status-warning",
        "#C69C5D",
        DtcgType::Color,
    ),
    token(
        "color.status-warning-bg",
        "status-warning-bg",
        "#2A2114",
        DtcgType::Color,
    ),
    token(
        "color.status-error",
        "status-error",
        "#B85052",
        DtcgType::Color,
    ),
    token(
        "color.status-error-bg",
        "status-error-bg",
        "#2A1818",
        DtcgType::Color,
    ),
    token(
        "color.status-info",
        "status-info",
        "#8EA1B8",
        DtcgType::Color,
    ),
    token(
        "color.status-info-bg",
        "status-info-bg",
        "#181D24",
        DtcgType::Color,
    ),
    token(
        "color.status-running",
        "status-running",
        "#B08E5C",
        DtcgType::Color,
    ),
    token(
        "color.status-running-bg",
        "status-running-bg",
        "#241D13",
        DtcgType::Color,
    ),
    token("dye.aima", "aima", "#B85052", DtcgType::Color),
    token("dye.aima-bg", "aima-bg", "#2A1818", DtcgType::Color),
    token("dye.aporia", "aporia", "#7AA582", DtcgType::Color),
    token("dye.aporia-bg", "aporia-bg", "#182018", DtcgType::Color),
    token(
        "dye.thanatochromia",
        "thanatochromia",
        "#6F5B8A",
        DtcgType::Color,
    ),
    token(
        "dye.thanatochromia-bg",
        "thanatochromia-bg",
        "#1F1A2A",
        DtcgType::Color,
    ),
    token("dye.natural", "natural", "#B07840", DtcgType::Color),
    token("dye.natural-bg", "natural-bg", "#221C14", DtcgType::Color),
    token(
        "color.role-user",
        "role-user",
        "var(--natural)",
        DtcgType::Color,
    ),
    token(
        "color.role-assistant",
        "role-assistant",
        "var(--aporia)",
        DtcgType::Color,
    ),
    token(
        "color.role-system",
        "role-system",
        "var(--thanatochromia)",
        DtcgType::Color,
    ),
    token("color.code-fg", "code-fg", "#d8c7a8", DtcgType::Color),
    token("color.code-bg", "code-bg", "#171510", DtcgType::Color),
    token(
        "color.code-lang",
        "code-lang",
        "var(--text-muted)",
        DtcgType::Color,
    ),
    token(
        "color.syntax-keyword",
        "syntax-keyword",
        "#c69c5d",
        DtcgType::Color,
    ),
    token(
        "color.syntax-string",
        "syntax-string",
        "#7aa582",
        DtcgType::Color,
    ),
    token(
        "color.syntax-comment",
        "syntax-comment",
        "#8f826d",
        DtcgType::Color,
    ),
    token(
        "color.syntax-function",
        "syntax-function",
        "#b08e5c",
        DtcgType::Color,
    ),
    token(
        "color.syntax-type",
        "syntax-type",
        "#8ea1b8",
        DtcgType::Color,
    ),
    token(
        "color.syntax-number",
        "syntax-number",
        "#b85052",
        DtcgType::Color,
    ),
    token(
        "color.syntax-operator",
        "syntax-operator",
        "#c8bda8",
        DtcgType::Color,
    ),
    token(
        "color.selection-bg",
        "selection-bg",
        "rgba(154, 123, 79, 0.36)",
        DtcgType::Color,
    ),
    token(
        "color.selection-fg",
        "selection-fg",
        "var(--text-primary)",
        DtcgType::Color,
    ),
    token(
        "color.scrollbar-track",
        "scrollbar-track",
        "var(--bg-surface-dim)",
        DtcgType::Color,
    ),
    token(
        "color.scrollbar-thumb",
        "scrollbar-thumb",
        "var(--border)",
        DtcgType::Color,
    ),
    token(
        "color.scrollbar-thumb-hover",
        "scrollbar-thumb-hover",
        "var(--accent-dim)",
        DtcgType::Color,
    ),
    token(
        "color.elevation-base",
        "elevation-base",
        "var(--bg)",
        DtcgType::Color,
    ),
    token(
        "color.elevation-sunken",
        "elevation-sunken",
        "var(--bg-surface-dim)",
        DtcgType::Color,
    ),
    token(
        "color.elevation-raised",
        "elevation-raised",
        "var(--bg-surface)",
        DtcgType::Color,
    ),
    token(
        "color.elevation-float",
        "elevation-float",
        "var(--bg-surface-bright)",
        DtcgType::Color,
    ),
];

const LIGHT_THEME_TOKENS: &[DtcgToken] = &[
    token("color.bg", "bg", "#F7F3E8", DtcgType::Color),
    token("color.bg-surface", "bg-surface", "#FFFCF4", DtcgType::Color),
    token(
        "color.bg-surface-bright",
        "bg-surface-bright",
        "#FFFFFF",
        DtcgType::Color,
    ),
    token(
        "color.bg-surface-dim",
        "bg-surface-dim",
        "#EEE6D6",
        DtcgType::Color,
    ),
    token(
        "color.bg-overlay",
        "bg-overlay",
        "rgba(44, 32, 22, 0.28)",
        DtcgType::Color,
    ),
    token(
        "color.bg-hover",
        "bg-hover",
        "var(--bg-surface-dim)",
        DtcgType::Color,
    ),
    token(
        "color.bg-elevated",
        "bg-elevated",
        "var(--bg-surface-bright)",
        DtcgType::Color,
    ),
    token(
        "color.text-primary",
        "text-primary",
        "#201A14",
        DtcgType::Color,
    ),
    token(
        "color.text-secondary",
        "text-secondary",
        "#5F5141",
        DtcgType::Color,
    ),
    token("color.text-muted", "text-muted", "#817260", DtcgType::Color),
    token(
        "color.text-inverse",
        "text-inverse",
        "#F7F3E8",
        DtcgType::Color,
    ),
    token("color.border", "border", "#D7C9B3", DtcgType::Color),
    token(
        "color.border-separator",
        "border-separator",
        "#E7DDCA",
        DtcgType::Color,
    ),
    token("color.input-bg", "input-bg", "#FFFCF4", DtcgType::Color),
    token(
        "color.input-border",
        "input-border",
        "#C8B89E",
        DtcgType::Color,
    ),
    token(
        "color.status-success",
        "status-success",
        "#4A7A52",
        DtcgType::Color,
    ),
    token(
        "color.status-success-bg",
        "status-success-bg",
        "#DEE8E0",
        DtcgType::Color,
    ),
    token(
        "color.status-warning",
        "status-warning",
        "#8B5A2B",
        DtcgType::Color,
    ),
    token(
        "color.status-warning-bg",
        "status-warning-bg",
        "#EFE3CF",
        DtcgType::Color,
    ),
    token(
        "color.status-error",
        "status-error",
        "#581523",
        DtcgType::Color,
    ),
    token(
        "color.status-error-bg",
        "status-error-bg",
        "#F0DDDE",
        DtcgType::Color,
    ),
    token(
        "color.status-info",
        "status-info",
        "#3E5874",
        DtcgType::Color,
    ),
    token(
        "color.status-info-bg",
        "status-info-bg",
        "#DEE6ED",
        DtcgType::Color,
    ),
    token(
        "color.status-running",
        "status-running",
        "#7A6340",
        DtcgType::Color,
    ),
    token(
        "color.status-running-bg",
        "status-running-bg",
        "#E9DFC9",
        DtcgType::Color,
    ),
    token("dye.aima", "aima", "#581523", DtcgType::Color),
    token("dye.aima-bg", "aima-bg", "#F0DDDE", DtcgType::Color),
    token("dye.aporia", "aporia", "#4A7A52", DtcgType::Color),
    token("dye.aporia-bg", "aporia-bg", "#DEE8E0", DtcgType::Color),
    token(
        "dye.thanatochromia",
        "thanatochromia",
        "#2C1B3A",
        DtcgType::Color,
    ),
    token(
        "dye.thanatochromia-bg",
        "thanatochromia-bg",
        "#E8DEED",
        DtcgType::Color,
    ),
    token("dye.natural", "natural", "#8B5A2B", DtcgType::Color),
    token("dye.natural-bg", "natural-bg", "#ECE0D0", DtcgType::Color),
    token(
        "color.role-user",
        "role-user",
        "var(--natural)",
        DtcgType::Color,
    ),
    token(
        "color.role-assistant",
        "role-assistant",
        "var(--aporia)",
        DtcgType::Color,
    ),
    token(
        "color.role-system",
        "role-system",
        "var(--thanatochromia)",
        DtcgType::Color,
    ),
    token("color.code-fg", "code-fg", "#3B2F23", DtcgType::Color),
    token("color.code-bg", "code-bg", "#EEE6D6", DtcgType::Color),
    token(
        "color.code-lang",
        "code-lang",
        "var(--text-muted)",
        DtcgType::Color,
    ),
    token(
        "color.syntax-keyword",
        "syntax-keyword",
        "#8B5A2B",
        DtcgType::Color,
    ),
    token(
        "color.syntax-string",
        "syntax-string",
        "#4A7A52",
        DtcgType::Color,
    ),
    token(
        "color.syntax-comment",
        "syntax-comment",
        "#817260",
        DtcgType::Color,
    ),
    token(
        "color.syntax-function",
        "syntax-function",
        "#7A6340",
        DtcgType::Color,
    ),
    token(
        "color.syntax-type",
        "syntax-type",
        "#3E5874",
        DtcgType::Color,
    ),
    token(
        "color.syntax-number",
        "syntax-number",
        "#581523",
        DtcgType::Color,
    ),
    token(
        "color.syntax-operator",
        "syntax-operator",
        "#5F5141",
        DtcgType::Color,
    ),
    token(
        "color.selection-bg",
        "selection-bg",
        "rgba(154, 123, 79, 0.24)",
        DtcgType::Color,
    ),
    token(
        "color.selection-fg",
        "selection-fg",
        "var(--text-primary)",
        DtcgType::Color,
    ),
    token(
        "color.scrollbar-track",
        "scrollbar-track",
        "var(--bg-surface-dim)",
        DtcgType::Color,
    ),
    token(
        "color.scrollbar-thumb",
        "scrollbar-thumb",
        "var(--border)",
        DtcgType::Color,
    ),
    token(
        "color.scrollbar-thumb-hover",
        "scrollbar-thumb-hover",
        "var(--accent-dim)",
        DtcgType::Color,
    ),
    token(
        "color.elevation-base",
        "elevation-base",
        "var(--bg)",
        DtcgType::Color,
    ),
    token(
        "color.elevation-sunken",
        "elevation-sunken",
        "var(--bg-surface-dim)",
        DtcgType::Color,
    ),
    token(
        "color.elevation-raised",
        "elevation-raised",
        "var(--bg-surface)",
        DtcgType::Color,
    ),
    token(
        "color.elevation-float",
        "elevation-float",
        "var(--bg-surface-bright)",
        DtcgType::Color,
    ),
];

const ROOT_GROUPS: &[TokenGroup] = &[
    TokenGroup {
        name: "color",
        tokens: COLOR_TOKENS,
    },
    TokenGroup {
        name: "typography",
        tokens: TYPOGRAPHY_TOKENS,
    },
    TokenGroup {
        name: "spacing",
        tokens: SPACING_TOKENS,
    },
    TokenGroup {
        name: "radii",
        tokens: RADIUS_TOKENS,
    },
    TokenGroup {
        name: "shadows",
        tokens: SHADOW_TOKENS,
    },
    TokenGroup {
        name: "motion",
        tokens: MOTION_TOKENS,
    },
    TokenGroup {
        name: "z-index",
        tokens: Z_INDEX_TOKENS,
    },
];

static STYLESHEET: OnceLock<String> = OnceLock::new();

pub(crate) fn stylesheet() -> &'static str {
    STYLESHEET.get_or_init(render_stylesheet).as_str()
}

const fn token(
    path: &'static str,
    name: &'static str,
    value: &'static str,
    token_type: DtcgType,
) -> DtcgToken {
    DtcgToken {
        path,
        name,
        value,
        token_type,
    }
}

fn render_stylesheet() -> String {
    let mut css = String::from("[data-harmonia-root] {\n");
    for group in ROOT_GROUPS {
        css.push_str("  /* DTCG group: ");
        css.push_str(group.name);
        css.push_str(" */\n");
        render_tokens(&mut css, group.tokens);
    }
    css.push_str("}\n\n[data-harmonia-root][data-theme=\"dark\"] {\n");
    render_tokens(&mut css, DARK_THEME_TOKENS);
    css.push_str("}\n\n[data-harmonia-root][data-theme=\"light\"] {\n");
    render_tokens(&mut css, LIGHT_THEME_TOKENS);
    css.push_str("}\n");
    css
}

fn render_tokens(css: &mut String, tokens: &[DtcgToken]) {
    for token in tokens {
        css.push_str("  --");
        css.push_str(token.name);
        css.push_str(": ");
        css.push_str(token.value);
        css.push_str("; /* ");
        css.push_str(token.path);
        css.push_str(" $type=");
        css.push_str(token.token_type.as_str());
        css.push_str(" */\n");
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    #[test]
    fn root_tokens_cover_required_scales() {
        assert!(COLOR_TOKENS.len() >= 8);
        assert!(
            TYPOGRAPHY_TOKENS
                .iter()
                .filter(|token| token.name.starts_with("text-"))
                .count()
                >= 6
        );
        assert!(SPACING_TOKENS.len() >= 8);
        assert!(SHADOW_TOKENS.len() >= 4);
        assert!(
            MOTION_TOKENS
                .iter()
                .filter(|token| token.name.starts_with("duration-"))
                .count()
                >= 3
        );
    }

    #[test]
    fn theme_tokens_cover_semantic_palette() {
        assert!(
            DARK_THEME_TOKENS
                .iter()
                .filter(|token| token.token_type == DtcgType::Color)
                .count()
                >= 12
        );
        assert!(
            LIGHT_THEME_TOKENS
                .iter()
                .filter(|token| token.token_type == DtcgType::Color)
                .count()
                >= 12
        );
    }

    #[test]
    fn dark_and_light_theme_names_match() {
        let dark_names: HashSet<_> = DARK_THEME_TOKENS.iter().map(|token| token.name).collect();
        let light_names: HashSet<_> = LIGHT_THEME_TOKENS.iter().map(|token| token.name).collect();
        assert_eq!(dark_names, light_names);
    }

    #[test]
    fn stylesheet_is_scoped_to_theme_provider_root() {
        let css = stylesheet();
        assert!(css.contains("[data-harmonia-root]"));
        assert!(css.contains("[data-harmonia-root][data-theme=\"dark\"]"));
        assert!(css.contains("[data-harmonia-root][data-theme=\"light\"]"));
        assert!(css.contains("--bg: #12110f"));
        assert!(css.contains("--bg: #F7F3E8"));
    }
}
