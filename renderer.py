from abc import ABC, abstractmethod

from osm import FeatureType, NodeProperties
from projection import Projection
from svg import Generator


class Renderer(ABC):
    @abstractmethod
    def draw_way(self):
        ...


class SVGRenderer(Renderer):
    def __init__(self, output_path, projection: Projection):
        self.output_path = output_path
        self.projection = projection
        self.svg = Generator(output_path)

    def draw_node(self, node, node_properties: NodeProperties):
        if node_properties.is_subway_stop():
            self.svg.underground_logo(*self.projection.transform(*node), size=200, z=5)

    def draw_way(self, way, feature_type: FeatureType):
        if feature_type == FeatureType.UNKNOWN or feature_type == FeatureType.BUILDING:
            return
        z, kwargs = self.get_rendering_args(feature_type)

        path = self.svg.path(z, **kwargs)
        for coords in way:
            path.node(*self.projection.transform(*coords))
        path.add()

    def draw_relation(self, ways, feature_type: FeatureType):
        if feature_type in [FeatureType.UNKNOWN, FeatureType.BUILDING]:
            return
        z, kwargs = self.get_rendering_args(feature_type)
        if feature_type == FeatureType.UNDERGROUND:
            ordered = [ways]
        else:
            ordered = self.reorder_ways(ways)
        for group in ordered:
            with self.svg.group(z):
                path = self.svg.path(z, **kwargs)
                for way in group:
                    for coords in way:
                        path.node(*self.projection.transform(*coords))
                path.add()

    def get_rendering_args(self, feature_type: FeatureType):
        if feature_type == FeatureType.WATER_BODY:
            kwargs = {'fill': 'lightblue', 'stroke': 'none'}
            z = 0
        elif feature_type == FeatureType.WATERWAY:
            kwargs = {'fill': 'none', 'stroke_width': 10, 'stroke': 'blue'}
            z = -1
        elif feature_type == FeatureType.RAILWAY:
            kwargs = {'fill': 'none', 'stroke_width': 10, 'stroke': 'grey'}
            z = 2
        elif feature_type == FeatureType.PARK:
            z = -1
            kwargs = {'fill': 'lightgreen', 'stroke': 'none'}
        elif feature_type == FeatureType.BUILDING:
            z = -1
            kwargs = {'fill': 'steelblue', 'stroke': 'none', 'stroke_width': 5}
        elif feature_type == FeatureType.UNDERGROUND:
            z = 2
            kwargs = {'fill': 'none', 'stroke': 'red', 'stroke_width': 20}
        else:
            kwargs = {'fill': 'none', 'stroke_width': 10, 'stroke': 'black'}
            z = 3
        return z, kwargs

    def reorder_ways(self, ways):
        def get_next(link_node, way):
            # if len(way) < 2:
                # raise Exception("Illegal state: way must consist of at least 2 nodes")
            if link_node == way[0]:
                next_link = way[-1]
            elif link_node == way[-1]:
                next_link = way[0]
            else:
                raise Exception("Illegal state: link_node should be start or end of way")
            candidates = by_end[next_link]
            if len(candidates) == 1:
                # Actually I don't think this can happen, will test later lol
                return None, None  # Self loop
            else:
                idx = 0
                while idx < len(candidates) and candidates[idx] == way:
                    idx += 1
                if idx == len(candidates):
                    # self-loop
                    return None, None
                else:
                    return next_link, candidates[idx]

        def find_unseen(seen, ways):
            for way in ways:
                if tuple(way) not in seen:
                    # print(f'find unseen({[hash(tuple(s)) for s in seen]}, {[hash(tuple(way)) for way in ways]}) = {hash(tuple(way))}')
                    return way
            raise Exception("Illegal state: all ways have been seen")

        if len(ways) < 1:
            raise Exception("Illegal state: 0-length way!")
        by_end = {}
        for way in ways:
            if way[0] not in by_end:
                by_end[way[0]] = []
            by_end[way[0]].append(way)
            if way[-1] not in by_end:
                by_end[way[-1]] = []
            by_end[way[-1]].append(way)

        ordered = [[ways[0]]]
        link_node = ordered[0][0][0]
        seen = set((tuple(ways[0]),))
        while sum(map(len, ordered)) < len(ways):
            link_node, next_way = get_next(link_node, ordered[-1][-1])
            if next_way is None or tuple(next_way) in seen:
                next_way = find_unseen(seen, ways)
                link_node = next_way[0]
                ordered.append([])
            # print(f' -> {hash(tuple(next_way))}', end='')
            ordered[-1].append(next_way)
            seen.add(tuple(next_way))
        # print()
        # print("A" + str([hash(tuple(way)) for group in ordered for way in group]))
        # print("B" + str([hash(tuple(way)) for way in ways]))
        return ordered

    def finalize(self):
        self.svg.write_svg(*self.projection.get_img_dims(), *self.projection.get_origin())
