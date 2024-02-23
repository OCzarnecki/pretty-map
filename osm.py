from enum import Enum, auto


class FeatureType(Enum):
    UNKNOWN = auto()
    RAILWAY = auto()
    HIGHWAY = auto()  # Probably needs to be broken down further
    WATER_BODY = auto()
    WATERWAY = auto()
    PARK = auto()
    BUILDING = auto()
    UNDERGROUND = auto()
    IGNORED = auto()


class NodeProperties():
    def __init__(self):
        self.props = set()

    def add(self, k, v):
        self.props.add(f'{k}:{v}')

    def _has(self, k, v):
        return f'{k}:{v}' in self.props

    def clear(self):
        self.props.clear()

    def is_subway_stop(self):
        return (self._has('railway', 'stop')
                and self._has('subway', 'yes')
                and self._has('public_transport', 'stop_position'))
