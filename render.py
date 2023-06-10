import argparse
import drawsvg as svg
import lzma
import json
import xml.sax

from abc import ABC, abstractmethod
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


class RenderingParser(xml.sax.handler.ContentHandler):
    def __init__(self, renderer):
        self.renderer = renderer
        self.known_elements = set()
        self.nodes = {}
        self.nodes_done = False
        self.inside_way = False
        self.current_way = []

    def startElement(self, name, attrs: xml.sax.xmlreader.AttributesImpl):
        if name == 'node':
            self.nodes[int(attrs.getValue('id'))] = (float(attrs.getValue('lon')), float(attrs.getValue('lat')))
            if self.nodes_done:
                raise OSMParseException('Got node after way!')
        # print(name, attrs.getNames())
        elif name == 'way':
            self.inside_way = True
        elif name == 'nd':
            if self.inside_way:
                self.current_way.append(self.nodes[int(attrs.getValue('ref'))])
        elif name == 'tag':
            # print('k', attrs.getValue('k'))
            # print('v', attrs.getValue('v'))
            pass

    def endElement(self, name):
        if name == 'way':
            self.emit_way()

    def emit_way(self):
        self.renderer.draw_way(self.current_way)
        self.current_way.clear()

    def print_bb(self):
        import numpy as np
        vals = np.array(list(self.nodes.values()))
        longs = vals[:, 0]
        lats = vals[:, 1]
        print(vals)
        print(f'lon ({min(longs)}, {max(longs)}) lat ({min(lats)}, {max(lats)})')


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
        # print(f'transformed lon {lon} lat {lat}')
        # print(f'    rel_lon {rel_lon} rel_lat {rel_lat}')
        # print(f'    u {u} v {v} x {x} y {x}')
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
        self.drawing = svg.Drawing(*projection.get_img_dims(), origin=projection.get_origin())
        self.projection = projection

    def draw_way(self, way):
        path = svg.Path(fill='none', stroke_width=1, stroke='black')
        # path.M(way[0][1] + 180, way[0][0] + 180)
        path.M(*self.projection.transform(*way[0]))
        idx = 1
        while idx < len(way):
            # path.L(way[idx][1] + 180, way[idx][0] + 180)
            path.L(*self.projection.transform(*way[idx]))
            idx += 1

        self.drawing.append(path)

    def save_drawing(self):
        if self.output_path.endswith('.png'):
            self.drawing.save_png(self.output_path)
        else:
            self.drawing.save_svg(self.output_path)


def render(config):
    xml_parser = xml.sax.make_parser()

    renderer = SVGRenderer(config.dest_path, Mercantor.from_config(config))
    renderin_parser = RenderingParser(renderer)
    xml_parser.setContentHandler(renderin_parser)

    with lzma.open(config.data_path) as fin:
        xml_parser.parse(fin)

    renderin_parser.print_bb()

    renderer.save_drawing()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('config', type=str, help="Path to json configuration file.")
    args = ap.parse_args()

    config = load_config(args.config)
    render(config)


if __name__ == '__main__':
    main()
