import cairosvg
import io
import math
import sys
import xml.etree.ElementTree as ET

from PIL import Image
from PyQt6 import QtCore, QtWidgets, QtSvg


def render_large(in_path, out_path, tile_size=1300):
    tree = ET.parse(in_path)
    root = tree.getroot()

    total_width = int(float(root.get("width"))) // 10
    total_height = int(float(root.get("height"))) // 10
    vb_x1, vb_y1, vb_x2, vb_y2 = map(float, root.get("viewBox").split())
    assert vb_x2 > vb_x1
    assert vb_y2 > vb_y1
    vb_width = vb_x2 - vb_x1
    vb_height = vb_y2 - vb_y1

    dest = Image.new("RGB", (total_width, total_height))

    x_tiles = math.ceil(total_width / tile_size)
    y_tiles = math.ceil(total_height / tile_size)

    for i in range(x_tiles):
        for j in range(y_tiles):
            tile_x = i * tile_size
            tile_y = j * tile_size
            tile_width = tile_size if i + 1 < x_tiles else total_width % tile_size
            tile_height = tile_size if j + 1 < y_tiles else total_height % tile_size

            tile_vb_x = vb_x1 + vb_width * tile_x / total_width
            tile_vb_y = vb_y1 + vb_height * tile_y / total_height
            tile_vb_width = vb_width * (tile_width) / total_width
            tile_vb_height = vb_height * (tile_height) / total_height

            print(f"Rendering tile {i},{j}: coords {tile_x} {tile_y} {tile_x + tile_width} {tile_y + tile_height}, viewbox {tile_vb_x} {tile_vb_y} {tile_vb_width} {tile_vb_height}...", end='', flush=True)

            root.set("width", str(tile_width))
            root.set("height", str(tile_height))
            root.set("viewBox", f"{tile_vb_x} {tile_vb_y} {tile_vb_width} {tile_vb_height}")
            tile = cairosvg.svg2png(
                bytestring=ET.tostring(root, encoding='utf8', method='xml'),
                background_color='white',
            )
            tile_img = Image.open(io.BytesIO(tile))
            print("Pasting...", end='', flush=True)
            dest.paste(tile_img, (tile_x, tile_y))
            print("Done!")
    dest.save(out_path)


def display(path):
    app = QtWidgets.QApplication(sys.argv)
    window = QtWidgets.QMainWindow()
    widget = QtSvg.QSvgWidget(path, window)
    widget.renderer().setAspectRatioMode(QtCore.Qt.AspectRatioMode(1))
    window.setCentralWidget(widget)
    window.show()
    sys.exit(app.exec_())


if __name__ == '__main__':
    # display(sys.argv[1])
    render_large(sys.argv[1], sys.argv[1].replace(".svg", ".png"))
