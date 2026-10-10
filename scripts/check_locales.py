#!/usr/bin/env python3
"""Check catalog keys, placeholders and source coverage without translating user data."""
import collections
import json
import pathlib
import re
import string

ROOT = pathlib.Path(__file__).resolve().parents[1]
CODES = ('ru', 'en', 'es', 'zh-Hans', 'hi', 'fr', 'ja', 'pt-BR', 'ar', 'ko')

def fields(text):
    result = collections.Counter()
    ordinal = 0
    for _, key, spec, _ in string.Formatter().parse(text):
        if key is not None:
            if not key:
                key = str(ordinal)
                ordinal += 1
            result[(key, spec)] += 1
    return result

def main():
    catalogs = {code: json.loads((ROOT / 'locales' / f'{code}.json').read_text()) for code in CODES}
    english = catalogs['en']
    assert catalogs['ru'].keys() == english.keys(), 'Russian and English keys must match'
    for code, catalog in catalogs.items():
        assert catalog.keys() == english.keys(), f'{code}: missing or unknown keys'
        for source, translated in catalog.items():
            assert translated, (code, source, 'empty translation')
            assert fields(source) == fields(translated), (code, source, 'placeholder mismatch', fields(translated))
            assert not any(ord(c) < 32 and c not in '\n\t' for c in translated), (code, source, 'control character')
    # Both t() literals and formatting templates must have an English fallback.
    literal = r'"(?:\\.|[^"\\])*"'
    for file in (ROOT / 'src').rglob('*.rs'):
        text = file.read_text()
        for match in re.finditer(r'\b(?:t|i18n::format)\(\s*(' + literal + ')', text):
            source = json.loads(match[1].replace('\n', '\\n'))
            assert source in english, (file.relative_to(ROOT), source, 'missing translation key')
    for code, catalog in catalogs.items():
        print(f'{code}: {len(catalog)} translations, keys and placeholders verified')

if __name__ == '__main__':
    main()
