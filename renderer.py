from abc import ABC, abstractmethod

from osm import WayType, RelationType
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

    def draw_way(self, way, way_type: WayType):
        if way_type == WayType.UNKNOWN:
            return
        z = 0
        if way_type == WayType.WATER_BODY:
            kwargs = {'fill': 'lightblue', 'stroke': 'none'}
            z = 0
        elif way_type == WayType.WATERWAY:
            kwargs = {'fill': 'none', 'stroke_width': 10, 'stroke': 'blue'}
            z = -1
        elif way_type == WayType.RAILWAY:
            kwargs = {'fill': 'none', 'stroke_width': 10, 'stroke': 'grey'}
            z = 2
        elif way_type == WayType.PARK:
            z = -1
            kwargs = {'fill': 'lightgreen', 'stroke': 'none'}
        else:
            kwargs = {'fill': 'none', 'stroke_width': 10, 'stroke': 'black'}
            z = 3

        path = self.svg.path(z, **kwargs)
        for coords in way:
            path.node(*self.projection.transform(*coords))
        path.add()

    def draw_relation(self, ways, relation_type: RelationType):
        if relation_type == relation_type.UNKNOWN:
            return
        if relation_type == RelationType.WATER_BODY:
            kwargs = {'fill': 'lightblue', 'stroke': 'none'}
            z = 0
        elif relation_type == RelationType.PARK:
            kwargs = {'fill': 'lightgreen', 'stroke': 'none'}
            z = -1
        ordered = self.reorder_ways(ways)
        for group in ordered:
            with self.svg.group(z):
                path = self.svg.path(z, **kwargs)
                for way in group:
                    for coords in way:
                        path.node(*self.projection.transform(*coords))
                path.add()

    def reorder_ways(self, ways):
        def get_next(link_node, way):
            if len(way) < 2:
                raise Exception("Illegal state: way must consist of at least 2 nodes")
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
#             elif len(candidates) == 2:
#                 if candidates[0] == candidates[1]:
#                     return None, None  # Self loop
#                 if way == candidates[0]:
#                     next_way = candidates[1]
#                 elif way == candidates[1]:
#                     next_way = candidates[0]
#                 else:
#                     raise Exception("Illegal state: current way must be one of the candidates")
#             else:
#                 raise Exception(f"Illegal state: there must always be one or two candidates, but there were {len(candidates)}")
#             return next_link, next_way

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
        # print([(k, [hash(tuple(c)) for c in v]) for k, v in by_end.items()])

        ordered = [[ways[0]]]
        link_node = ordered[0][0][0]
        seen = set((tuple(ways[0]),))
        # print(hash(tuple(ways[0])), end='')
        while sum(map(len, ordered)) < len(ways):
            link_node, next_way = get_next(link_node, ordered[-1][-1])
            if next_way is None or tuple(next_way) in seen:
                # we've looped around
                # print(f' D({hash(tuple(next_way)) if next_way is not None else None}) ', end='')
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
