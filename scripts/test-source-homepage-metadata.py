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
        # Actual App counterexample: a description default must not replace the
        # canonical name fallback for an unsupported UI language (ja-JP).
        self.assertEqual(metadata.extract({'localizedNames': names}), {'localizedNames': names})
        description_only = metadata.extract({'localizedNames': names, 'localization': {
            'version': 1, 'defaultLocale': 'zh-Hans',
            'messages': {'source.description': {'zh-Hans': '漫畫目錄'}}}})
        self.assertEqual(description_only['localizedNames'], names)
        self.assertNotIn('name', description_only['presentation']['texts'])
        self.assertEqual(description_only['presentation']['texts']['description'], {'zh-Hans': '漫畫目錄'})
        for invalid in [None, [], {'en': ' padded '}, {'en': 'X', 'EN': 'Y'}, {'bad tag': 'X'}]:
            with self.subTest(names=invalid), self.assertRaises(ValueError):
                metadata.extract({'localizedNames': invalid})

    def test_localization_projects_one_catalog_without_auth_or_identity_mutation(self):
        raw = json.loads((ROOT / 'sources/komiic/metadata.json').read_text())
        projected = metadata.extract(raw)
        catalog = raw['localization']
        resources = projected['presentation']
        self.assertEqual(resources['defaultLocale'], catalog['defaultLocale'])
        self.assertEqual(projected['localizedNames'], catalog['messages']['source.name'])
        self.assertEqual(resources['texts']['name'], projected['localizedNames'])
        self.assertEqual(resources['texts']['description'], catalog['messages']['source.description'])
        for role in ['authentication.usernameLabel', 'authentication.passwordLabel']:
            self.assertEqual(resources['texts'][role], catalog['messages'][role])
        self.assertEqual(projected['authentication'], raw['authentication'])
        self.assertNotIn('localization', projected)
        self.assertEqual(set(resources['texts']), {'name', 'description',
            'authentication.usernameLabel', 'authentication.passwordLabel'})
        no_auth = metadata.extract({'localization': catalog})
        self.assertNotIn('authentication', no_auth)
        self.assertEqual(set(no_auth['presentation']['texts']), {'name', 'description'})
        # Run the real production manifest AND index expressions, not a test copy.
        expressions = [line.strip().removesuffix(' \\').strip("'")
                       for line in (ROOT / 'build.sh').read_text().splitlines()
                       if line.strip().startswith("'{id: $id")]
        for expression in expressions:
            command = ['jq', '-n']
            for name in ['id', 'name', 'version', 'lang', 'author', 'description',
                         'contentRating', 'minAppVersion', 'icon', 'pkg', 'sha256']:
                command += ['--arg', name, '' if name == 'description' else 'canonical']
            command += ['--argjson', 'nsfw', 'false', '--argjson', 'sourceMetadata', json.dumps(projected),
                        '--argjson', 'webSessionOrigins', '[]', '--argjson', 'distribution', '{}', expression]
            actual = json.loads(subprocess.check_output(command, text=True))
            self.assertEqual(actual['name'], 'canonical')
            self.assertEqual(actual['description'], '')
            self.assertEqual(actual['presentation'], resources)
            self.assertEqual(actual['localizedNames'], projected['localizedNames'])
            self.assertNotIn('localization', actual)

    def test_localization_optional_failures_and_legacy_conflicts(self):
        catalog = {'version': 1, 'defaultLocale': 'zh-Hant', 'messages': {
            'source.name': {'zh-Hant': '漫畫', 'en': 'Comics', 'bad tag': 'skip'},
            'source.description': {'zh-Hant': '第一行\n第二行', 'en': '\ud800'},
            'authentication.usernameLabel': {'en': 'User'}}}
        actual = metadata.extract({'localization': catalog})
        self.assertEqual(actual['localizedNames'], {'zh-Hant': '漫畫', 'en': 'Comics'})
        self.assertEqual(actual['presentation']['texts']['description'], {'zh-Hant': '第一行\n第二行'})
        self.assertNotIn('authentication.usernameLabel', actual['presentation']['texts'])
        with self.assertRaisesRegex(ValueError, 'conflicts'):
            metadata.extract({'localization': catalog, 'localizedNames': {'EN': 'Other'}})
        for value in [None, {'version': 2, 'messages': catalog['messages']}]:
            self.assertEqual(metadata.extract({'localization': value}), {})
        duplicate = json.loads('{"localization":{"version":1,"messages":{"source.name":{"en":"A","EN":"B"}}}}',
                               object_pairs_hook=metadata.MetadataObject)
        self.assertEqual(metadata.extract(duplicate), {})
        # Raw count is bounded before duplicates collapse in json.loads.
        over = json.loads('{"localization":{"version":1,"messages":{"source.name":{' +
                          ','.join(['"en":"A"'] * 17) + '}}}}', object_pairs_hook=metadata.MetadataObject)
        self.assertEqual(metadata.extract(over), {})

    def test_localized_author_projects_without_overwriting_canonical_author(self):
        raw = {'localization': {'version': 1, 'defaultLocale': 'zh-Hant', 'messages': {
            'source.author': {'zh-Hant': '來源作者', 'zh-Hans': '来源作者', 'en': 'Source author'}}}}
        projected = metadata.extract(raw)
        self.assertEqual(projected['presentation']['texts'], {'author': raw['localization']['messages']['source.author']})
        self.assertNotIn('author', projected)
        expressions = [line.strip().removesuffix(' \\').strip("'")
                       for line in (ROOT / 'build.sh').read_text().splitlines()
                       if line.strip().startswith("'{id: $id")]
        for expression in expressions:
            command = ['jq', '-n']
            for name in ['id', 'name', 'version', 'lang', 'author', 'description',
                         'contentRating', 'minAppVersion', 'icon', 'pkg', 'sha256']:
                command += ['--arg', name, 'Canonical author' if name == 'author' else 'canonical']
            command += ['--argjson', 'nsfw', 'false', '--argjson', 'sourceMetadata', json.dumps(projected),
                        '--argjson', 'webSessionOrigins', '[]', '--argjson', 'distribution', '{}', expression]
            actual = json.loads(subprocess.check_output(command, text=True))
            self.assertEqual(actual['author'], 'Canonical author')
            self.assertEqual(actual['presentation'], projected['presentation'])
        self.assertEqual(metadata.extract({'author': 'Canonical author'}), {})
        for invalid in ['x' * 129, '🙂' * 65, ' padded ', 'line\nnext', '\ud800']:
            with self.subTest(author=repr(invalid)):
                self.assertEqual(metadata.extract({'localization': {'version': 1, 'messages': {
                    'source.author': {'en': invalid}}}}), {})

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
                if path.parent.name in {'goda', 'mangadex', 'manhuaren'}:
                    # User correction and the actual GoDa official homepage:
                    # navigation must not inherit the retained request backend.
                    # 2026-10-27: MangaDex title URL and mangaapi.manhuaren.com
                    # similarly do not define their official homepage spelling.
                    frontends = {'goda': 'https://godamh.com/',
                                 'mangadex': 'https://mangadex.org/',
                                 'manhuaren': 'https://www.manhuaren.com/'}
                    self.assertEqual(value, frontends[path.parent.name])
                    projected = metadata.extract({**raw,
                        'requestBaseUrl': 'https://baozimh.org',
                        'webSessionOrigins': ['https://baozimh.org']})
                    self.assertEqual(projected['homepage'], frontends[path.parent.name])
                    self.assertNotIn('requestBaseUrl', projected)
                    self.assertNotIn('webSessionOrigins', projected)
                    continue
                self.assertTrue('"' + value + '"' in source
                                or '\\"' + value + '\\"' in source,
                                'homepage must retain its complete reviewed source literal')
                self.assertEqual(metadata.extract(raw)['homepage'], value)

    def test_production_cli_requires_homepage_without_changing_legacy_projection(self):
        missing = ROOT / 'sources/example-demo/no-metadata.json'
        command = [sys.executable, str(ROOT / 'scripts/read-source-localized-metadata.py'), str(missing)]
        self.assertEqual(json.loads(subprocess.check_output(command, text=True)), {})
        result = subprocess.run(command + ['--require-homepage'], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('requires an explicit official homepage', result.stderr)
        for path in (ROOT / 'sources').glob('*/metadata.json'):
            projected = json.loads(subprocess.check_output(command[:2] + [str(path), '--require-homepage'], text=True))
            self.assertEqual(projected['homepage'], json.loads(path.read_text())['homepage'])

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


    def test_chapter_identity_projects_to_the_same_legacy_rule(self):
        for name in ['baozimh', 'mh1234']:
            raw = json.loads((ROOT / 'sources' / name / 'metadata.json').read_text())
            chapter = raw['identity']['chapter']
            old = {key: value for key, value in raw.items() if key != 'identity'}
            old['identity'] = {key: value for key, value in raw['identity'].items() if key != 'chapter'}
            old['websitePolicy'] = {**old.get('websitePolicy', {}), 'chapterIdentity': chapter}
            with self.subTest(source=name):
                self.assertEqual(metadata.extract(raw), metadata.extract(old))
                with self.assertRaises(ValueError):
                    metadata.extract({**old, 'identity': raw['identity']})
                with self.assertRaises(ValueError):
                    metadata.extract({**raw, 'identity': {'chapter': {'workPrefix': 'manga:'}}})

if __name__ == '__main__':
    unittest.main()
