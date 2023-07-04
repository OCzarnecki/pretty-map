import argparse
import drawsvg as svg
import lzma
import json
import xml.sax

from abc import ABC, abstractmethod
from enum import Enum, auto
from typing import Tuple


class Config:
    def __init__(self, data_path, dest_path, top_left_lon, top_left_lat, px_per_deg, width_px, height_px):
        self.data_path = data_path
        self.dest_path = dest_path
        self.top_left_lon = top_left_lon
        self.top_left_lat = top_left_lat
        self.px_per_deg = px_per_deg
        self.width_px = width_px
        self.height_px = height_px


def load_config(path):
    with open(path) as f:
        config_json = json.load(f)
    return Config(**config_json)


class OSMParseException(Exception):
    pass


class WayType(Enum):
    UNKNOWN = auto()
    RAILWAY = auto()
    HIGHWAY = auto()  # Probably needs to be broken down further
    WATER_BODY = auto()
    WATERWAY = auto()


class RenderingParser(xml.sax.handler.ContentHandler):
    def __init__(self, renderer):
        self.renderer = renderer
        self.known_elements = set()
        self.nodes = {}
        self.nodes_done = False
        self.current_way = []
        self.way_type = None

    def startElement(self, name, attrs: xml.sax.xmlreader.AttributesImpl):
        if name == 'node':
            self.nodes[int(attrs.getValue('id'))] = (float(attrs.getValue('lon')), float(attrs.getValue('lat')))
            if self.nodes_done:
                raise OSMParseException('Got node after way!')
        # print(name, attrs.getNames())
        elif name == 'way':
            self.way_type = WayType.UNKNOWN
        elif name == 'nd':
            if self.way_type is not None:
                self.current_way.append(self.nodes[int(attrs.getValue('ref'))])
        elif name == 'tag':
            if self.way_type is not None:
                k = attrs.getValue('k')
                v = attrs.getValue('v')
                if k == 'railway':
                    self.way_type = WayType.RAILWAY
                elif k == 'highway':
                    self.way_type = WayType.HIGHWAY
                elif k == 'water':
                    self.way_type = WayType.WATER_BODY
                elif k == 'waterway':
                    self.way_type = WayType.WATERWAY
                if self.way_type == WayType.UNKNOWN:
                    # print('k', k)
                    # print('v', v)
                    # print()
                    pass

    def endElement(self, name):
        if name == 'way':
            self.emit_way()

    def emit_way(self):
        self.renderer.draw_way(self.current_way, self.way_type)
        self.current_way.clear()
        self.way_type = None

    def print_bb(self):
        import numpy as np
        vals = np.array(list(self.nodes.values()))
        longs = vals[:, 0]
        lats = vals[:, 1]
        print(f'lon ({min(longs)}, {max(longs)}) lat ({min(lats)}, {max(lats)})')

    def draw_bb(self):
        import numpy as np
        vals = np.array(list(self.nodes.values()))
        longs = vals[:, 0]
        lats = vals[:, 1]
        self.renderer.draw_box(min(longs), min(lats), max(longs), max(lats))


class Projection(ABC):
    @abstractmethod
    def get_origin(self) -> Tuple[float, float]:
        ...

    @abstractmethod
    def get_img_dims(self) -> Tuple[float, float]:
        ...

    @abstractmethod
    def transform(self, lon, lat) -> Tuple[float, float]:
        ...


class Mercantor(Projection):
    def __init__(self, tl_lon, tl_lat, px_per_deg, width_px, height_px):
        self.tl_lon = tl_lon
        self.tl_lat = tl_lat
        self.px_per_deg = px_per_deg
        self.width_px = width_px
        self.height_px = height_px

    def get_origin(self):
        return (0.0, 0.0)

    def get_img_dims(self):
        return (self.width_px, self.height_px)

    def transform(self, lon, lat):
        rel_lon = lon - self.tl_lon
        rel_lat = lat - self.tl_lat

        x = rel_lon * self.px_per_deg
        y = - rel_lat * self.px_per_deg
        return (x, y)

    @staticmethod
    def from_config(config: Config):
        return Mercantor(
            config.top_left_lon,
            config.top_left_lat,
            config.px_per_deg,
            config.width_px,
            config.height_px,
        )


class Renderer(ABC):
    @abstractmethod
    def draw_way(self):
        ...


class SVGRenderer(Renderer):
    def __init__(self, output_path, projection: Projection):
        self.output_path = output_path
        self.projection = projection
        self.elements = {}

    def draw_way(self, way, way_type):
        if way_type == WayType.UNKNOWN:
            return
        z = 0
        if way_type == WayType.WATER_BODY:
            kwargs = {'fill': 'blue', 'stroke_width': 1, 'stroke': 'none'}
            z = 0
        elif way_type == WayType.WATERWAY:
            kwargs = {'fill': 'none', 'stroke_width': 1, 'stroke': 'blue'}
            z = 1
        elif way_type == WayType.RAILWAY:
            kwargs = {'fill': 'none', 'stroke_width': 1, 'stroke': 'grey'}
            z = 2
        else:
            kwargs = {'fill': 'none', 'stroke_width': 1, 'stroke': 'black'}
            z = 3

        path = svg.Path(**kwargs)
        path.M(*self.projection.transform(*way[0]))
        idx = 1
        while idx < len(way):
            path.L(*self.projection.transform(*way[idx]))
            idx += 1

        self._add_element(path, z)

    def finalize(self):
        drawing = svg.Drawing(*self.projection.get_img_dims(), origin=self.projection.get_origin())
        zs = sorted(self.elements.keys())
        for z in zs:
            for el in self.elements[z]:
                drawing.append(el)

        if self.output_path.endswith('.png'):
            drawing.save_png(self.output_path)
        else:
            drawing.save_svg(self.output_path)

    def draw_box(self, min_lon, min_lat, max_lon, max_lat):
        min_x, min_y = self.projection.transform(min_lon, min_lat)
        max_x, max_y = self.projection.transform(max_lon, max_lat)
        path = svg.Path(fill='none', stroke_width=3, stroke='red')
        path.M(min_x, min_y).L(min_x, max_y).L(max_x, max_y).L(max_x, min_y).L(min_x, min_y)
        self.drawing.append(path)

    def _add_element(self, el, z=0):
        if z not in self.elements:
            self.elements[z] = []
        self.elements[z].append(el)


def render(config):
    xml_parser = xml.sax.make_parser()

    renderer = SVGRenderer(config.dest_path, Mercantor.from_config(config))
    rendering_parser = RenderingParser(renderer)
    xml_parser.setContentHandler(rendering_parser)

    with lzma.open(config.data_path) as fin:
        xml_parser.parse(fin)

    # rendering_parser.print_bb()
    # rendering_parser.draw_bb()

    renderer.finalize()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('config', type=str, help="Path to json configuration file.")
    args = ap.parse_args()

    config = load_config(args.config)
    render(config)


if __name__ == '__main__':
    main()
