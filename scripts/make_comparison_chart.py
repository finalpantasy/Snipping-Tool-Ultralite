"""Writes docs/media/comparison-light.svg and comparison-dark.svg.

Values are medians from docs/PERFORMANCE.md, measured October 14, 2026 on one
Windows 11 PC. Each row has its own scale; lower is better everywhere.
None means not measured. Run: python scripts/make_comparison_chart.py
"""

from pathlib import Path

APPS = ["Snipping Tool UL", "Windows Snipping Tool", "ShareX", "Greenshot", "Flameshot"]

ROWS = [
    # label, detail, unit, values in APPS order, word for "lowest"
    ("Start a snip", "hotkey to selection screen", "ms", [170, 363, 249, 265, None], "fastest"),
    ("Image on clipboard", "after you let go of the mouse", "ms", [50, 91, 144, 63, None], "fastest"),
    ("Memory while snipping", "RAM in use during a capture", "MB", [14, 474, 922, 133, None], "least"),
    ("Memory in the background", "waiting in the tray", "MB", [2.0, 0, 107, 45, 8.3], "least"),
    ("Space on disk", "unpacked or installed size", "MB", [9.6, 420, 517, 9.0, 95], "smallest"),
]

NOTES = {(3, 1): "0 MB, not running", (2, 4): "not measured", (0, 4): "not measured", (1, 4): "not measured"}

THEMES = {
    "light": dict(ul="#00849E", other="#9AA3AD", text="#1F2328", muted="#59636E", rule="#D1D9E0"),
    "dark": dict(ul="#24A3BA", other="#6A727C", text="#E6EDF3", muted="#9198A1", rule="#3D444D"),
}

FONT = "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif"
WIDTH, NAME_X, BAR_X, BAR_MAX, TOP = 880, 232, 390, 280, 60
BAR_H, GAP, ROW_PAD = 12, 4, 30
ORDINALS = ["", "2nd ", "3rd ", "4th ", "5th "]


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
    text = f"{v:,.1f}" if isinstance(v, float) and v < 20 else f"{round(v):,}"
    return f"{text} {unit}"


def row_height():
    return len(APPS) * (BAR_H + GAP) - GAP + ROW_PAD


def chart(t):
    height = TOP + row_height() * len(ROWS) + 4
    desc = "; ".join(
        f"{label}: " + ", ".join(f"{a} {fmt(v, unit) if v is not None else 'not measured'}" for a, v in zip(APPS, vals))
        for label, _, unit, vals, _ in ROWS
    )
    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{height}" '
        f'viewBox="0 0 {WIDTH} {height}" font-family="{FONT}" role="img" aria-labelledby="t d">',
        '<title id="t">Snipping Tool UL compared with Windows Snipping Tool, ShareX, Greenshot and Flameshot</title>',
        f'<desc id="d">{desc}. Lower is better.</desc>',
        f'<rect x="{NAME_X}" y="16" width="12" height="12" rx="2" fill="{t["ul"]}"/>',
        f'<text x="{NAME_X + 18}" y="26.5" font-size="14" font-weight="600" fill="{t["text"]}">Snipping Tool UL</text>',
        f'<rect x="{NAME_X + 160}" y="16" width="12" height="12" rx="2" fill="{t["other"]}"/>',
        f'<text x="{NAME_X + 178}" y="26.5" font-size="14" fill="{t["text"]}">Other snipping tools</text>',
        f'<text x="{WIDTH - 8}" y="26.5" font-size="13" fill="{t["muted"]}" text-anchor="end">Lower is better</text>',
    ]
    for i, (label, detail, unit, vals, word) in enumerate(ROWS):
        y = TOP + i * row_height()
        top = max(v for v in vals if v is not None)
        if i:
            out.append(f'<line x1="0" x2="{WIDTH}" y1="{y - 14}" y2="{y - 14}" stroke="{t["rule"]}" stroke-width="1"/>')
        out.append(f'<text x="0" y="{y + 13}" font-size="15" font-weight="600" fill="{t["text"]}">{label}</text>')
        out.append(f'<text x="0" y="{y + 32}" font-size="12.5" fill="{t["muted"]}">{detail}</text>')
        for j, (app, v) in enumerate(zip(APPS, vals)):
            by = y + j * (BAR_H + GAP)
            ul = j == 0
            ink, weight = (t["text"], "600") if ul else (t["muted"], "400")
            out.append(
                f'<text x="{BAR_X - 10}" y="{by + 10.5}" font-size="12.5" font-weight="{weight}" '
                f'fill="{ink}" text-anchor="end">{app}</text>'
            )
            note = NOTES.get((i, j))
            if v is None or (note and v == 0):
                out.append(f'<text x="{BAR_X}" y="{by + 10.5}" font-size="12.5" font-style="italic" fill="{t["muted"]}">{note}</text>')
                continue
            w = v / top * BAR_MAX
            out.append(bar(BAR_X, by, w, t["ul"] if ul else t["other"]))
            out.append(
                f'<text x="{BAR_X + max(w, 3) + 8:.1f}" y="{by + 10.5}" font-size="12.5" '
                f'font-weight="{weight}" fill="{ink}">{fmt(v, unit)}</text>'
            )
        measured = sorted(v for v in vals if v is not None)
        rank = measured.index(vals[0])
        out.append(
            f'<text x="{WIDTH - 8}" y="{y + 22}" font-size="15" font-weight="700" '
            f'fill="{t["text"]}" text-anchor="end">UL: {ORDINALS[rank]}{word}</text>'
        )
    out.append("</svg>")
    return "\n".join(out) + "\n"


if __name__ == "__main__":
    media = Path(__file__).resolve().parent.parent / "docs" / "media"
    for name, theme in THEMES.items():
        (media / f"comparison-{name}.svg").write_text(chart(theme), encoding="utf-8")
        print(f"wrote comparison-{name}.svg")
