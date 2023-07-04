import json


class Config:
    def __init__(self, data_path, dest_path, top_left_lon, top_left_lat, px_per_deg, width_px, height_px):
        self.data_path = data_path
        self.dest_path = dest_path
        self.top_left_lon = top_left_lon
        self.top_left_lat = top_left_lat
        self.px_per_deg = px_per_deg
        self.width_px = width_px
        self.height_px = height_px

    @staticmethod
    def load_config(path):
        with open(path) as f:
            config_json = json.load(f)
        return Config(**config_json)
