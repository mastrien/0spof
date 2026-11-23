"""Gera os ícones do app Android a partir de `crates/spof-desktop/icons/icon.png`."""

from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
SRC_ICON = ROOT / "crates/spof-desktop/icons/icon.png"
RES_DIR = ROOT / "mobile/android/app/src/main/res"

LEGACY_SIZES = {
    "mipmap-mdpi": 48,
    "mipmap-hdpi": 72,
    "mipmap-xhdpi": 96,
    "mipmap-xxhdpi": 144,
    "mipmap-xxxhdpi": 192,
}

ADAPTIVE_SIZES = {
    "mipmap-mdpi": 108,
    "mipmap-hdpi": 162,
    "mipmap-xhdpi": 216,
    "mipmap-xxhdpi": 324,
    "mipmap-xxxhdpi": 432,
}

BG_COLOR = (23, 35, 63)  # #17233F
FG_COLOR = (246, 247, 251)  # #F6F7FB


def extract_glyph(icon: Image.Image) -> Image.Image:
    """Extrai o glifo claro de dentro do círculo #17233F com anti-aliasing limpo."""
    rgba = icon.convert("RGBA")
    w, h = rgba.size
    cx, cy = (w - 1) / 2.0, (h - 1) / 2.0
    max_r2 = 196.0 * 196.0

    src_px = rgba.load()
    glyph = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    dst_px = glyph.load()

    dr = FG_COLOR[0] - BG_COLOR[0]
    dg = FG_COLOR[1] - BG_COLOR[1]
    db = FG_COLOR[2] - BG_COLOR[2]

    for y in range(h):
        dy = y - cy
        for x in range(w):
            dx = x - cx
            if dx * dx + dy * dy > max_r2:
                continue
            r, g, b, a = src_px[x, y]
            if a < 16:
                continue
            tr = max(0.0, (r - BG_COLOR[0]) / dr)
            tg = max(0.0, (g - BG_COLOR[1]) / dg)
            tb = max(0.0, (b - BG_COLOR[2]) / db)
            t = min(1.0, (tr + tg + tb) / 3.0)
            alpha = int(round(255.0 * t))
            if alpha > 0:
                dst_px[x, y] = (FG_COLOR[0], FG_COLOR[1], FG_COLOR[2], alpha)

    return glyph


def place_adaptive(glyph: Image.Image, canvas_size: int, rgb: tuple[int, int, int]) -> Image.Image:
    """Centraliza o glifo na proporção 83,2% do canvas de 108dp (dentro da zona segura de 66dp)."""
    scaled_size = round(canvas_size * 0.832)
    resized = glyph.resize((scaled_size, scaled_size), Image.Resampling.LANCZOS)
    if rgb != FG_COLOR:
        r, g, b = rgb
        _, _, _, a = resized.split()
        colored = Image.new("RGBA", resized.size, (r, g, b, 0))
        colored.putalpha(a)
        resized = colored

    canvas = Image.new("RGBA", (canvas_size, canvas_size), (0, 0, 0, 0))
    offset = (canvas_size - scaled_size) // 2
    canvas.paste(resized, (offset, offset), resized)
    return canvas


def main() -> None:
    src = Image.open(SRC_ICON).convert("RGBA")
    # Recorta margem transparente excessiva (bbox 51..461) mantendo ~3.4% de respiro.
    legacy_crop = src.crop((36, 36, 476, 476))
    glyph = extract_glyph(src)

    for folder, px in LEGACY_SIZES.items():
        out_dir = RES_DIR / folder
        out_dir.mkdir(parents=True, exist_ok=True)
        legacy_crop.resize((px, px), Image.Resampling.LANCZOS).save(out_dir / "ic_launcher.png")

    for folder, px in ADAPTIVE_SIZES.items():
        out_dir = RES_DIR / folder
        out_dir.mkdir(parents=True, exist_ok=True)
        place_adaptive(glyph, px, FG_COLOR).save(out_dir / "ic_launcher_foreground.png")
        place_adaptive(glyph, px, (255, 255, 255)).save(out_dir / "ic_launcher_monochrome.png")

    print("Ícones Android gerados com sucesso em", RES_DIR)


if __name__ == "__main__":
    main()
