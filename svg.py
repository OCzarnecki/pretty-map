from typing import Dict


class Generator:
    layers: Dict[float, bytes]

    def __init__(self, output_path):
        self.output_path = output_path
        self.layers = {}

    def _add_element(self, el: bytes, z=0):
        if z not in self.layers:
            self.layers[z] = []
        self.layers[z] += el

    def write_svg(self, width, height):
        with open(self.output_path, 'wb') as of:
            of.write(f'''
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
     width="{width}" height="{height}" viewBox="0.0 0.0 {width} {height}">
<defs>
</defs>
                     '''.encode('utf8'))

            zs = sorted(self.layers.keys())
            for z in zs:
                of.write(self.layers[z])

