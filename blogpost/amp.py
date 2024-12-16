from dataclasses import dataclass, field
import pickle

import lzma
import xml.sax

import os

import hashlib

PATH = "../planet_-0.418,51.37_0.268,51.647.osm.xz"


class StopCollection(Exception):
    pass


@dataclass(frozen=True)
class Node:
    id_: int
    lat: float
    lon: float


@dataclass(frozen=True)
class WayDesc:
    id_: int
    node_refs: list[int]


@dataclass(frozen=True)
class RelationDesc:
    id_: int
    way_refs: list[int]
    node_refs: list[int]


@dataclass
class NodeCollector(xml.sax.handler.ContentHandler):
    ids: list[int]
    collected: dict[int, WayDesc] = field(default_factory=dict)

    def startElement(self, name: str, attrs: xml.sax.xmlreader.AttributesImpl):
        if name == "node":
            id_ = int(attrs.getValue("id"))
            if id_ in self.ids:
                lat = float(attrs.getValue("lat"))
                lon = float(attrs.getValue("lon"))
                self.collected[id_] = Node(
                    id_,
                    lat,
                    lon,
                )
        elif name == "way":
            raise StopCollection()


@dataclass
class WayCollector(xml.sax.handler.ContentHandler):
    ids: list[int]
    collected: dict[int, WayDesc] = field(default_factory=dict)
    currently_collecting: int | None = None
    node_refs: list[int] = field(default_factory=list)

    def startElement(self, name: str, attrs: xml.sax.xmlreader.AttributesImpl):
        if name == "way":
            id_ = int(attrs.getValue("id"))
            if id_ in self.ids:
                self.currently_collecting = id_
        elif name == "nd":
            ref = int(attrs.getValue('ref'))
            self.node_refs.append(ref)
        elif name == "relation":
            raise StopCollection()

    def endElement(self, name: str):
        if name == "way":
            if self.currently_collecting is not None:
                self.collected[self.currently_collecting] = WayDesc(
                    self.currently_collecting,
                    self.node_refs,
                )
            self.currently_collecting = None
            self.node_refs = []
            self.way_refs = []


@dataclass
class RelationsCollector(xml.sax.handler.ContentHandler):
    ids: list[int]
    collected: dict[int, RelationDesc] = field(default_factory=dict)
    currently_collecting: int | None = None
    node_refs: list[int] = field(default_factory=list)
    way_refs: list[int] = field(default_factory=list)

    def startElement(self, name: str, attrs: xml.sax.xmlreader.AttributesImpl):
        if name == "relation":
            id_ = int(attrs.getValue("id"))
            if id_ in self.ids:
                self.currently_collecting = id_
        elif name == "member":
            ref = int(attrs.getValue('ref'))
            if attrs.getValue("type") == "way":
                self.way_refs.append(ref)
            elif attrs.getValue("type") == "node":
                self.node_refs.append(ref)

    def endElement(self, name: str):
        if name == "relation":
            if self.currently_collecting is not None:
                self.collected[self.currently_collecting] = RelationDesc(
                    self.currently_collecting,
                    self.way_refs,
                    self.node_refs,
                )
            self.currently_collecting = None
            self.node_refs = []
            self.way_refs = []


def collect_nodes(ids):
    key = ".cache/nodes" + hashlib.sha256(",".join(sorted(list(map(str, ids)))).encode("utf8")).hexdigest() + ".pickle"
    if os.path.exists(key):
        with open(key, "rb") as f:
            return pickle.load(f)

    print(ids)

    xml_parser = xml.sax.make_parser()

    collector = NodeCollector(ids=ids)
    xml_parser.setContentHandler(collector)

    try:
        with lzma.open(PATH) as fin:
            xml_parser.parse(fin)
    except StopCollection:
        pass

    collected = collector.collected
    os.makedirs(os.path.dirname(key), exist_ok=True)
    with open(key, "wb") as f:
        pickle.dump(collected, f)
    return collected


def collect_ways(ids):
    key = ".cache/ways" + hashlib.sha256(",".join(sorted(list(map(str, ids)))).encode("utf8")).hexdigest() + ".pickle"
    if os.path.exists(key):
        with open(key, "rb") as f:
            return pickle.load(f)

    xml_parser = xml.sax.make_parser()

    collector = WayCollector(ids=ids)
    xml_parser.setContentHandler(collector)

    try:
        with lzma.open(PATH) as fin:
            xml_parser.parse(fin)
    except StopCollection:
        pass

    collected = collector.collected
    os.makedirs(os.path.dirname(key), exist_ok=True)
    with open(key, "wb") as f:
        pickle.dump(collected, f)
    return collected


def collect_relations(ids):
    key = ".cache/relations" + hashlib.sha256(",".join(sorted(list(map(str, ids)))).encode("utf8")).hexdigest() + ".pickle"
    if os.path.exists(key):
        with open(key, "rb") as f:
            return pickle.load(f)

    xml_parser = xml.sax.make_parser()

    collector = RelationsCollector(ids=ids)
    xml_parser.setContentHandler(collector)

    with lzma.open(PATH) as fin:
        xml_parser.parse(fin)

    collected = collector.collected
    os.makedirs(os.path.dirname(key), exist_ok=True)
    with open(key, "wb") as f:
        pickle.dump(collected, f)
    return collected


def plot():
    ...


def main():
    relations = collect_relations(ids=[28934])
    ways = collect_ways(
        [way_ref for relation in relations.values() for way_ref in relation.way_refs]
    )
    nodes = collect_nodes(
        [node_ref for relation in relations.values() for node_ref in relation.node_refs]
        + [node_ref for way in ways.values() for node_ref in way.node_refs]
    )


if __name__ == "__main__":
    main()
