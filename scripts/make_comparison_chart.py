"""Writes docs/media/comparison-light.svg and comparison-dark.svg.

Values are medians from docs/PERFORMANCE.md (UL 0.3.22 vs Microsoft Snipping
Tool 11.2608.41.0). Each row has its own scale; lower is better everywhere.
Run: python scripts/make_comparison_chart.py
"""

from pathlib import Path

ROWS = [
    # label, detail, UL value, Windows value, unit, verdict word
    ("Start a snip", "hotkey to selection screen", 155, 355, "ms", "faster"),
    ("Image on clipboard", "after you let go of the mouse", 50, 76, "ms", "faster"),
    ("Open the app", "launch to window", 47, 235, "ms", "faster"),
    ("Memory while snipping", "RAM in use during a capture", 29, 484, "MB", "less"),
    ("Space on disk", "installed size", 10, 420, "MB", "smaller"),
]

THEMES = {
    "light": dict(ul="#00849E", win="#9AA3AD", text="#1F2328", muted="#59636E", rule="#D1D9E0"),
    "dark": dict(ul="#24A3BA", win="#6A727C", text="#E6EDF3", muted="#9198A1", rule="#3D444D"),
}

FONT = "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif"
WIDTH, LABEL_W, BAR_X, BAR_MAX, ROW_H, TOP = 880, 250, 262, 420, 70, 64
BAR_H, GAP = 16, 2


def bar(x, y, w, color):
    """Horizontal bar anchored at the baseline with a 4px rounded data end."""
    w = max(w, 3)
    if w < 8:
        return f'<rect x="{x}" y="{y}" width="{w:.1f}" height="{BAR_H}" fill="{color}"/>'
    r = 4
    return (
        f'<path d="M{x},{y} h{w - r:.1f} a{r},{r} 0 0 1 {r},{r} v{BAR_H - 2 * r} '
        f'a{r},{r} 0 0 1 -{r},{r} h-{w - r:.1f} z" fill="{color}"/>'
    )


def fmt(v, unit):
    return f"{v:,} {unit}"


def chart(t):
    height = TOP + ROW_H * len(ROWS) + 16
    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{height}" '
        f'viewBox="0 0 {WIDTH} {height}" font-family="{FONT}" role="img" aria-labelledby="t d">',
        '<title id="t">Snipping Tool UL compared with Windows Snipping Tool</title>',
        '<desc id="d">'
        + "; ".join(f"{r[0]}: UL {fmt(r[2], r[4])}, Windows {fmt(r[3], r[4])}" for r in ROWS)
        + ". Lower is better.</desc>",
        # Legend
        f'<rect x="{BAR_X}" y="18" width="12" height="12" rx="2" fill="{t["ul"]}"/>',
        f'<text x="{BAR_X + 18}" y="28.5" font-size="14" fill="{t["text"]}" font-weight="600">Snipping Tool UL</text>',
        f'<rect x="{BAR_X + 160}" y="18" width="12" height="12" rx="2" fill="{t["win"]}"/>',
        f'<text x="{BAR_X + 178}" y="28.5" font-size="14" fill="{t["text"]}">Windows Snipping Tool</text>',
        f'<text x="{WIDTH - 8}" y="28.5" font-size="13" fill="{t["muted"]}" text-anchor="end">Lower is better</text>',
    ]
    for i, (label, detail, ul, win, unit, word) in enumerate(ROWS):
        y = TOP + i * ROW_H
        scale = BAR_MAX / max(ul, win)
        if i:
            out.append(f'<line x1="0" x2="{WIDTH}" y1="{y - 12}" y2="{y - 12}" stroke="{t["rule"]}" stroke-width="1"/>')
        out.append(f'<text x="0" y="{y + 14}" font-size="15" font-weight="600" fill="{t["text"]}">{label}</text>')
        out.append(f'<text x="0" y="{y + 34}" font-size="12.5" fill="{t["muted"]}">{detail}</text>')
        for j, (v, color, weight, ink) in enumerate(
            [(ul, t["ul"], "600", t["text"]), (win, t["win"], "400", t["muted"])]
        ):
            by = y + j * (BAR_H + GAP)
            w = v * scale
            out.append(bar(BAR_X, by, w, color))
            out.append(
                f'<text x="{BAR_X + max(w, 3) + 8:.1f}" y="{by + 12.5}" font-size="13" '
                f'font-weight="{weight}" fill="{ink}">{fmt(v, unit)}</text>'
            )
        ratio = win / ul
        ratio_text = f"{ratio:.1f}× {word}" if ratio < 10 else f"{ratio:.0f}× {word}"
        out.append(
            f'<text x="{WIDTH - 8}" y="{y + 22}" font-size="15" font-weight="700" '
            f'fill="{t["text"]}" text-anchor="end">{ratio_text}</text>'
        )
    out.append("</svg>")
    return "\n".join(out) + "\n"


if __name__ == "__main__":
    media = Path(__file__).resolve().parent.parent / "docs" / "media"
    for name, theme in THEMES.items():
        (media / f"comparison-{name}.svg").write_text(chart(theme), encoding="utf-8")
        print(f"wrote comparison-{name}.svg")
