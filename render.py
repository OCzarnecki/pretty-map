import argparse
import lzma
import xml.sax

from config import Config
from osm import FeatureType, NodeProperties
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
        self.current_node_id = None
        self.current_node_properties = NodeProperties()
        self.current_way = []
        self.current_way_id = None
        self.feature_type = None
        self.current_relation_ways = []

    def startElement(self, name, attrs: xml.sax.xmlreader.AttributesImpl):
        if name == 'node':
            self.current_node_id = int(attrs.getValue('id'))
            self.nodes[self.current_node_id] = (float(attrs.getValue('lon')), float(attrs.getValue('lat')))

            if self.nodes_done:
                raise OSMParseException('Got node after way!')
        elif name == 'way':
            if not self.nodes_done:
                self.nodes_done = True
            self.feature_type = FeatureType.UNKNOWN
            self.current_way_id = int(attrs.getValue('id'))
        elif name == 'relation':
            self.feature_type = FeatureType.UNKNOWN
        elif name == 'nd':
            if self.feature_type is not None:
                self.current_way.append(self.nodes[int(attrs.getValue('ref'))])
        elif name == 'member':
            if attrs.getValue('type') == 'way':
                ref = int(attrs.getValue('ref'))
                if ref in self.ways:
                    self.current_relation_ways.append(self.ways[ref])
        elif name == 'tag':
            k = attrs.getValue('k')
            vs = attrs.getValue('v')
            for v in vs.split(';'):
                if self.feature_type is not None:
                    if k == 'railway' and v == 'rail':
                        self.feature_type = FeatureType.RAILWAY
                    elif k == 'highway':
                        self.feature_type = FeatureType.HIGHWAY
                    elif k == 'water':
                        self.feature_type = FeatureType.WATER_BODY
                    elif k == 'waterway' and self.feature_type == FeatureType.UNKNOWN:
                        # Only set waterway if we don't have another way_type. This is because
                        self.feature_type = FeatureType.WATERWAY
                    elif k == 'building':
                        self.feature_type = FeatureType.BUILDING
                    elif k == 'leisure' and v == 'park':
                        self.feature_type = FeatureType.PARK
                    elif k == 'railway' and v == 'subway':
                        self.feature_type = FeatureType.UNDERGROUND
                elif self.current_node_id is not None:
                    self.current_node_properties.add(k, v)

    def endElement(self, name):
        if name == 'way':
            self.emit_way()
        elif name == 'relation':
            self.emit_relation()
        elif name == 'node':
            self.emit_node()

    def emit_node(self):
        self.renderer.draw_node(self.nodes[self.current_node_id], self.current_node_properties)
        self.current_node_id = None
        self.current_node_properties.clear()

    def emit_way(self):
        self.renderer.draw_way(self.current_way, self.feature_type)
        self.ways[self.current_way_id] = self.current_way.copy()
        self.current_way.clear()
        self.feature_type = None
        self.current_way_id = None

    def emit_relation(self):
        if len(self.current_relation_ways) > 0:
            self.renderer.draw_relation(self.current_relation_ways, self.feature_type)
            self.current_relation_ways.clear()
        self.feature_type = None

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
