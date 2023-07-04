from abc import ABC, abstractmethod

from osm import WayType
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

        path = self.svg.path(z, **kwargs)
        for coords in way:
            path.node(*self.projection.transform(*coords))
        path.add()

    def finalize(self):
        self.svg.write_svg(*self.projection.get_img_dims(), *self.projection.get_origin())
