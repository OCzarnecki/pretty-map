from enum import Enum, auto


class WayType(Enum):
    UNKNOWN = auto()
    RAILWAY = auto()
    HIGHWAY = auto()  # Probably needs to be broken down further
    WATER_BODY = auto()
    WATERWAY = auto()
    PARK = auto()


class RelationType(Enum):
    UNKNOWN = auto()
    WATER_BODY = auto()
    PARK = auto()
