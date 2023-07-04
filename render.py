import argparse
import lzma
import xml.sax

from config import Config
from osm import WayType
from projection import Mercantor
from renderer import SVGRenderer


class OSMParseException(Exception):
    pass


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

    config = Config.load_config(args.config)
    render(config)


if __name__ == '__main__':
    main()
