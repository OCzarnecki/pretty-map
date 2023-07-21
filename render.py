import argparse
import lzma
import xml.sax

from config import Config
from osm import RelationType, WayType
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
        self.ways = {}
        self.current_way = []
        self.current_way_id = None
        self.way_type = None
        self.current_relation_ways = []
        self.relation_type = None

    def startElement(self, name, attrs: xml.sax.xmlreader.AttributesImpl):
        if name == 'node':
            self.nodes[int(attrs.getValue('id'))] = (float(attrs.getValue('lon')), float(attrs.getValue('lat')))
            if self.nodes_done:
                raise OSMParseException('Got node after way!')
        # print(name, attrs.getNames())
        elif name == 'way':
            if not self.nodes_done:
                print('nodes done!')
                self.nodes_done = True
            self.way_type = WayType.UNKNOWN
            self.current_way_id = int(attrs.getValue('id'))
        elif name == 'relation':
            self.relation_type = RelationType.UNKNOWN
        elif name == 'nd':
            if self.way_type is not None:
                self.current_way.append(self.nodes[int(attrs.getValue('ref'))])
        elif name == 'member':
            if attrs.getValue('type') == 'way':
                ref = int(attrs.getValue('ref'))
                if ref in self.ways:
                    self.current_relation_ways.append(self.ways[ref])
        elif name == 'tag':
            k = attrs.getValue('k')
            v = attrs.getValue('v')
            # if k in ['natural', 'leisure', 'man_made', 'boundary', 'highway', 'water', 'waterway']:
                # print(f"{k}:{v}")
            if self.way_type is not None:
                if k == 'railway':
                    self.way_type = WayType.RAILWAY
                elif k == 'highway':
                    self.way_type = WayType.HIGHWAY
                elif k == 'water':
                    self.way_type = WayType.WATER_BODY
                elif k == 'waterway' and self.way_type == WayType.UNKNOWN:
                    self.way_type = WayType.WATERWAY
                elif k == 'leisure' and v == 'park':
                    self.way_type = WayType.PARK
            elif self.relation_type is not None:
                if k == 'water':
                    self.relation_type = RelationType.WATER_BODY
                if k == 'leisure' and v == 'park':
                    self.relation_type = RelationType.PARK

    def endElement(self, name):
        if name == 'way':
            self.emit_way()
        elif name == 'relation':
            self.emit_relation()

    def emit_way(self):
        if self.way_type == WayType.UNKNOWN:
            self.ways[self.current_way_id] = self.current_way.copy()
        else:
            self.renderer.draw_way(self.current_way, self.way_type)
        self.current_way.clear()
        self.way_type = None
        self.current_way_id = None

    def emit_relation(self):
        if len(self.current_relation_ways) > 0:
            self.renderer.draw_relation(self.current_relation_ways, self.relation_type)
            self.current_relation_ways.clear()
        self.relation_type = None

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
