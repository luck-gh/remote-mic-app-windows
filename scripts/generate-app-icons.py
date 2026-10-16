"""从唯一设计母版 public/app-logo-personal.svg 确定性生成 Windows 图标。

需要现有 Python Pillow、Node 和 Sharp（SVG 渲染由 librsvg 提供）。不联网，
不安装依赖，不修改母版或已确认预览。通过 --node 与 --node-modules 指定
宿主已有工具；省略时使用 PATH 与 NODE_PATH。--check 只核验，不写资源。
--evidence-dir target/dev/personal-branding 输出可复现的浅深色/DPI 视觉板。
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import io
import json
import os
import pathlib
import subprocess
import sys

import PIL
from PIL import Image, ImageDraw

REPOSITORY_ROOT = pathlib.Path(__file__).resolve().parent.parent
ICON_DIRECTORY = REPOSITORY_ROOT / "src-tauri" / "icons" / "app-icons"
PUBLIC_DIRECTORY = REPOSITORY_ROOT / "public"
TRAY_SIZES = (16, 20, 24, 32)
ICO_SIZES = (16, 20, 24, 32, 40, 48, 64, 96, 128, 256)
SOURCE = PUBLIC_DIRECTORY / "app-logo-personal.svg"
SOURCE_HASH = "40cc8382d2e16ef060abac300d5f3fb9bfaf48ece841939d2eeac3151d9d238e"
BUNDLE_SIZES = {
    "32x32.png": 32, "128x128.png": 128, "128x128@2x.png": 256,
}


def render(node: str, node_modules: str | None) -> tuple[dict[int, Image.Image], dict]:
    sizes = sorted(set(BUNDLE_SIZES.values()) | set(ICO_SIZES) | {512})
    environment = os.environ.copy()
    if node_modules:
        environment["NODE_PATH"] = node_modules
    javascript = """
const fs = require('fs'); const sharp = require('sharp');
(async () => {
  const svg = fs.readFileSync(process.argv[1]);
  const images = {};
  for (const size of JSON.parse(process.argv[2])) {
    images[size] = (await sharp(svg, {density: 384})
      .resize(size, size, {kernel: 'lanczos3'}).png().toBuffer()).toString('base64');
  }
  process.stdout.write(JSON.stringify({versions: sharp.versions, images}));
})().catch(error => {process.stderr.write(error.message); process.exit(1);});
"""
    result = subprocess.run(
        [node, "-e", javascript, str(SOURCE), json.dumps(sizes)],
        env=environment, check=True, capture_output=True, text=True, timeout=120,
    )
    payload = json.loads(result.stdout)
    images = {
        int(size): Image.open(io.BytesIO(base64.b64decode(data))).convert("RGBA")
        for size, data in payload["images"].items()
    }
    return images, payload["versions"]


def asset_bytes(images: dict[int, Image.Image]) -> dict[pathlib.Path, bytes]:
    targets = {ICON_DIRECTORY / f"personal-{size}.png": size for size in (*TRAY_SIZES, 256)}
    targets.update({ICON_DIRECTORY.parent / name: size for name, size in BUNDLE_SIZES.items()})
    targets.update({PUBLIC_DIRECTORY / "app-logo.png": 512, PUBLIC_DIRECTORY / "favicon.png": 32})
    assets = {}
    for path, size in targets.items():
        stream = io.BytesIO()
        images[size].save(stream, format="PNG", optimize=True)
        assets[path] = stream.getvalue()
    stream = io.BytesIO()
    images[256].save(stream, format="ICO", sizes=[(s, s) for s in ICO_SIZES],
                     append_images=[images[s] for s in ICO_SIZES if s != 256])
    assets[ICON_DIRECTORY.parent / "icon.ico"] = stream.getvalue()
    return assets


def validate(assets: dict[pathlib.Path, bytes], images: dict[int, Image.Image]) -> None:
    for path, expected in assets.items():
        if not path.is_file() or path.read_bytes() != expected:
            raise ValueError(f"资源与母版生成结果不一致：{path.relative_to(REPOSITORY_ROOT)}")
        with Image.open(path) as image:
            if path.suffix == ".ico":
                if image.ico.sizes() != {(s, s) for s in ICO_SIZES}:
                    raise ValueError("ICO 尺寸条目不完整")
                for size in ICO_SIZES:
                    if image.ico.getimage((size, size)).convert("RGBA").tobytes() != images[size].tobytes():
                        raise ValueError(f"ICO {size}px 内容与 PNG 不一致")
            else:
                if image.mode != "RGBA" or image.getbbox() is None:
                    raise ValueError(f"非有效 RGBA 图标：{path.name}")
                alpha = image.getchannel("A")
                bounds = alpha.point(lambda value: 255 if value > 32 else 0).getbbox()
                if not bounds:
                    raise ValueError(f"图标边界无效：{path.name}")
                if any(image.getpixel(point)[3] for point in ((0, 0), (image.width - 1, 0), (0, image.height - 1), (image.width - 1, image.height - 1))):
                    raise ValueError(f"图标圆角透明边界丢失：{path.name}")


def evidence(directory: pathlib.Path, images: dict[int, Image.Image]) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    for theme, color, text_color in (("light", "#F4F4F4", "#161616"), ("dark", "#202020", "#EEEEEE")):
        board = Image.new("RGB", (1000, 430), color)
        draw = ImageDraw.Draw(board)
        draw.text((20, 12), f"Personal logo / {theme} / native pixels + 8x nearest-neighbor", fill=text_color)
        for index, size in enumerate(TRAY_SIZES):
            x = 20 + index * 240
            draw.text((x, 45), f"{size}px / {round(size / 16 * 100)}% DPI", fill=text_color)
            board.paste(images[size], (x, 80), images[size])
            expanded = images[size].resize((size * 8, size * 8), Image.Resampling.NEAREST)
            board.paste(expanded, (x, 140), expanded)
        board.save(directory / f"icon-optical-{theme}.png")
    board = Image.new("RGB", (1040, 550), "#F4F4F4")
    draw = ImageDraw.Draw(board)
    for index, size in enumerate((32, 48, 64, 128, 256)):
        x = 20 + index * 170
        draw.text((x, 10), f"{size}px / shell", fill="#161616")
        board.paste(images[size], (x, 45), images[size])
        tile = Image.new("RGB", (max(150, size), max(150, size)), "#202020")
        tile.paste(images[size], (0, 0), images[size])
        board.paste(tile, (x, 290))
    board.save(directory / "icon-optical-shell.png")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--node", default="node")
    parser.add_argument("--node-modules")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--evidence-dir", type=pathlib.Path)
    arguments = parser.parse_args()

    if hashlib.sha256(SOURCE.read_bytes()).hexdigest() != SOURCE_HASH:
        raise ValueError("设计母版与已确认 SHA256 不一致；请先确认新的设计母版")
    images, versions = render(arguments.node, arguments.node_modules)
    assets = asset_bytes(images)
    if not arguments.check:
        for path, data in assets.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    validate(assets, images)
    if arguments.evidence_dir:
        evidence(arguments.evidence_dir, images)
    print(json.dumps({"result": "passed", "mode": "check" if arguments.check else "generate",
                      "assets": len(assets), "master_sha256": SOURCE_HASH,
                      "ico_sizes": ICO_SIZES, "pillow": PIL.__version__,
                      "sharp": versions["sharp"], "librsvg": versions["rsvg"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
