from typing import Dict


class Generator:
    layers: Dict[float, bytes]

    def __init__(self, output_path):
        self.output_path = output_path
        self.layers = {}
        self.underground_logo_template = None

    def _add_str(self, z, el: bytes):
        if z not in self.layers:
            self.layers[z] = []
        self.layers[z] += el

    def _add_element(self, z, name: str, closed: bool = True, **kwargs):
        el_str = []
        el_str += b'<' + name.encode('utf8')
        for k, v in kwargs.items():
            el_str += b' ' + k.replace('_', '-').encode('utf8') + b'="'
            el_str += v
            el_str += b'"'
        if closed:
            el_str += b'/>\n'
        else:
            el_str += b'>\n'
        self._add_str(z, el_str)

    def _close_element(self, z, name: str):
        self._add_str(z, b'</' + name.encode('utf8') + b'>\n')

    def path(self, z=0, **kwargs):
        return SVGPath(self, z, kwargs)

    def underground_logo(self, x, y, size, z=0):
        if self.underground_logo_template is None:
            # lazy load
            self.underground_logo_template = open('./underground_logo.svg', 'r', encoding='utf8').read()

        logo = (self.underground_logo_template.replace('$$X$$', str(x))
                .replace('$$Y$$', str(y))
                .replace('$$WIDTH$$', str(size))
                .replace('$$HEIGHT$$', str(size)))
        self._add_str(z, logo.encode('utf8'))

    def write_svg(self, width, height, origin_x, origin_y):
        with open(self.output_path, 'wb') as of:
            of.write(f'''<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
     width="{width}" height="{height}" viewBox="{origin_x} {origin_y} {origin_x + width} {origin_y + height}">
<defs>
</defs>\n'''.encode('utf8'))

            zs = sorted(self.layers.keys())
            for z in zs:
                of.write(bytes(self.layers[z]))

            of.write(b'</svg>')

    def group(self, z, **kwargs):
        return GroupCM(self, z, kwargs)


class GroupCM:
    def __init__(self, generator, z, kwargs):
        self.z = z
        self.kwargs = kwargs
        self.generator = generator

    def __enter__(self):
        self.generator._add_element(self.z, 'g', closed=False, **self.kwargs)

    def __exit__(self, type, value, tb):
        if tb is None:
            self.generator._close_element(self.z, 'g')
        else:
            print(type, value, tb)
            raise Exception(type, value, tb)


class SVGPath:
    def __init__(self, generator, z, kwargs):
        self.d = []
        self.generator = generator
        self.z = z
        self.kwargs = {}
        for k, v in kwargs.items():
            self.kwargs[k] = str(v).encode('utf8')

    def node(self, x: float, y: float):
        if len(self.d) == 0:
            self.d += f'M{x:.2f},{y:.2f}'.encode('utf8')
        else:
            self.d += f' L{x:.2f},{y:.2f}'.encode('utf8')

    def add(self):
        self.generator._add_element(self.z, 'path', d=bytes(self.d), **self.kwargs)
