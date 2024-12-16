# first line: 102
@mem.cache(cache_validation_callback=lambda meta: True)
def collect_relations(ids, v=V_RELATIONS):
    xml_parser = xml.sax.make_parser()

    collector = RelationsCollector(ids=[28934])
    xml_parser.setContentHandler(collector)

    with lzma.open(PATH) as fin:
        xml_parser.parse(fin)

    return collector.collected
