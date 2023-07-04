from abc import ABC, abstractmethod
from typing import Tuple

from config import Config


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
    """Not a true mercantor projection. Doesn't handle any edge cases, but works for London.."""

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
