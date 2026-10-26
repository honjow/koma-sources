#!/usr/bin/env python3
"""Offline metadata/production jq projection behavior; no provider or build IO."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import unittest

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('metadata', ROOT / 'scripts/read-source-localized-metadata.py')
metadata = importlib.util.module_from_spec(spec)
spec.loader.exec_module(metadata)


class HomepageMetadataTest(unittest.TestCase):
    def test_old_metadata_and_explicit_names_contract(self):
        self.assertEqual(metadata.extract({}), {})
        self.assertEqual(metadata.extract({'localizedNames': {}}), {'localizedNames': {}})
        names = {'zh-Hans': '漫画', 'zh-Hant': '漫畫', 'en': 'Comic'}
        self.assertEqual(metadata.extract({'localizedNames': names}), {'localizedNames': names})
        for invalid in [None, [], {'en': ' padded '}, {'en': 'X', 'EN': 'Y'}, {'bad tag': 'X'}]:
            with self.subTest(names=invalid), self.assertRaises(ValueError):
                metadata.extract({'localizedNames': invalid})

    def test_safe_navigation_urls_remain_exact_and_do_not_require_names(self):
        for value in ['https://example.org/', 'http://localhost:8080/path',
                      'HTTPS://Example.org/path?q=1#anchor', 'https://例子.测试/漫画',
                      'https://example.org/%E6%BC%AB%E7%94%BB', 'https://[::1]:8080/']:
            with self.subTest(url=value):
                self.assertEqual(metadata.extract({'homepage': value}), {'homepage': value})

    def test_unsafe_navigation_rejected(self):
        for value in [None, 42, '', ' https://example.org', 'https://example.org/a b',
                      'javascript:alert(1)', '//example.org', 'file:///tmp/a', 'https:///path',
                      'https://user:password@example.org/', 'https://user@example.org/',
                      'https://example.org:wrong/', 'https://example.org:99999/',
                      'https://example.org\\@other.org/', 'https://example.org/\npath',
                      'https://exa%2fmple.org/', 'https://[invalid]/', 'https://example.org/"x']:
            with self.subTest(url=value), self.assertRaises(ValueError):
                metadata.extract({'homepage': value})

    def test_unknown_fields_never_become_metadata_or_session_grants(self):
        value = {'homepage': 'https://example.org/', 'webSessionOrigins': ['https://other.org'],
                 'permissions': {'network': True}, 'icon': 'override.png', 'id': 'other'}
        self.assertEqual(metadata.extract(value), {'homepage': 'https://example.org/'})

    def test_three_named_homepages_and_all_existing_names_read(self):
        expected = {'baozimh': 'https://www.baozimh.com/',
                    'manhuawu': 'https://www.mhua5.com/',
                    'manhuagui': 'https://www.manhuagui.com/'}
        for path in (ROOT / 'sources').glob('*/metadata.json'):
            raw = json.loads(path.read_text())
            actual = metadata.extract(raw)
            if 'localizedNames' in raw:
                self.assertEqual(actual['localizedNames'], raw['localizedNames'])
            if path.parent.name in expected:
                self.assertEqual(actual['homepage'], expected[path.parent.name])

    def test_additional_homepages_quote_complete_source_webpage_literals(self):
        # Source ownership/website-vs-API semantics are reviewed separately.
        # This protects the reviewed literal paths, queries and fragments from
        # being silently shortened into inferred site roots during packaging.
        previously_reviewed = {'baozimh', 'manhuawu', 'manhuagui'}
        for path in (ROOT / 'sources').glob('*/metadata.json'):
            raw = json.loads(path.read_text())
            if 'homepage' not in raw or path.parent.name in previously_reviewed:
                continue
            source = (path.parent / 'src/lib.rs').read_text()
            value = raw['homepage']
            with self.subTest(source=path.parent.name):
                if path.parent.name == 'goda':
                    # User correction and the actual GoDa official homepage:
                    # navigation must not inherit the retained request backend.
                    self.assertEqual(value, 'https://godamh.com/')
                    projected = metadata.extract({**raw,
                        'requestBaseUrl': 'https://baozimh.org',
                        'webSessionOrigins': ['https://baozimh.org']})
                    self.assertEqual(projected['homepage'], 'https://godamh.com/')
                    self.assertNotIn('requestBaseUrl', projected)
                    self.assertNotIn('webSessionOrigins', projected)
                    continue
                self.assertTrue('"' + value + '"' in source
                                or '\\"' + value + '\\"' in source,
                                'homepage must retain its complete reviewed source literal')
                self.assertEqual(metadata.extract(raw)['homepage'], value)

    def test_actual_manifest_and_index_jq_expressions_keep_homepage_and_separate_origins(self):
        # Execute the real production expressions, without build.sh's compilation,
        # dist cleanup, source_info runner, package generation or network calls.
        expressions = [line.strip().removesuffix(' \\').strip("'")
                       for line in (ROOT / 'build.sh').read_text().splitlines()
                       if line.strip().startswith("'{id: $id")]
        self.assertEqual(len(expressions), 2)
        for names in [False, True]:
            raw = {'homepage': 'https://example.org/'}
            if names:
                raw['localizedNames'] = {'zh-Hans': '示例'}
            projected = metadata.extract(raw)
            for expression in expressions:
                command = ['jq', '-n']
                for name in ['id', 'name', 'version', 'lang', 'author', 'description',
                             'contentRating', 'minAppVersion', 'icon', 'pkg']:
                    command += ['--arg', name, 'fixture']
                command += ['--argjson', 'nsfw', 'false', '--argjson', 'sourceMetadata', json.dumps(projected),
                            '--argjson', 'webSessionOrigins', '["https://session.example.org"]', expression]
                actual = json.loads(subprocess.check_output(command, text=True))
                self.assertEqual(actual['homepage'], raw['homepage'])
                self.assertEqual(actual.get('localizedNames'), raw.get('localizedNames'))
                if 'webSessionOrigins' in expression:
                    self.assertEqual(actual['webSessionOrigins'], ['https://session.example.org'])
                else:
                    self.assertNotIn('webSessionOrigins', actual)


if __name__ == '__main__':
    unittest.main()
