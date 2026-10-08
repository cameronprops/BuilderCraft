#!/usr/bin/env python3
"""Seed Rhino manual-topic coverage from downloaded public indexes, without help text."""
import argparse
import hashlib
import json
from datetime import date
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urljoin, urlsplit, urlunsplit

class Links(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links = []
    def handle_starttag(self, tag, attrs):
        if tag == 'a':
            href = dict(attrs).get('href')
            if href:
                self.links.append(href)

def main():
    args = argparse.ArgumentParser(description=__doc__)
    args.add_argument('--index', action='append', required=True)
    args.add_argument('--output', type=Path, required=True)
    values = args.parse_args()
    old = json.loads(values.output.read_text()) if values.output.exists() else {}
    reviewed = {x['url']: x for x in old.get('removed_topics', []) + old.get('topics', [])}
    topics = {}
    snapshots = []
    for item in values.index:
        platform, filename = item.split('=', 1)
        if platform not in ('windows', 'mac'):
            raise ValueError('unsupported platform')
        segment = '8' if platform == 'windows' else '8mac'
        base = f'https://docs.mcneel.com/rhino/{segment}/help/en-us/'
        index = base + 'commandlist/command_list.htm'
        raw = Path(filename).read_bytes()
        parser = Links()
        parser.feed(raw.decode('utf-8'))
        snapshots.append({'platform': platform, 'url': index, 'sha256': hashlib.sha256(raw).hexdigest()})
        found = set()
        for href in parser.links:
            url = urlsplit(urljoin(index, href))
            canonical = urlunsplit((url.scheme, url.netloc, url.path, '', ''))
            if canonical.startswith(base) and url.path.endswith('.htm'):
                path = url.path[len(urlsplit(base).path):]
                entry = {'url': canonical, 'platform': platform, 'path': path,
                         'topic_family': path.split('/')[0], 'review_status': 'not_reviewed',
                         'contract': None, 'native_services': [], 'acceptance_evidence': []}
                entry.update({k: v for k, v in reviewed.get(canonical, {}).items()
                              if k in ('review_status', 'contract', 'native_services', 'acceptance_evidence')})
                topics[canonical] = entry
                found.add(canonical)
        if len(found) < 500:
            raise ValueError("Unexpected index structure; refuse incomplete manual register")
    result = {'schema_version': 1, 'baseline': 'Rhino 8 stable major, rolling Windows/Mac help',
              'retrieved_at': date.today().isoformat(), 'minor_build': 'not_verified',
              'release_reference': 'https://www.rhino3d.com/download/',
              'coverage': 'All unique local manual topic links reachable directly from the command indexes. Full TOC/topic crawl and option review pending; not a complete manual inventory.',
              'policy': 'Reference behavior only. No proprietary implementation, help text, or icons copied.',
              'source_snapshots': snapshots, 'topic_count': len(topics),
              'topics': [topics[k] for k in sorted(topics)],
              'removed_topics': [reviewed[k] for k in sorted(set(reviewed)-set(topics))]}
    values.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'topic_count': len(topics), 'review_status': 'seeded; full manual traversal pending'}))
if __name__ == '__main__':
    main()
