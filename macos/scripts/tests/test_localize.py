# SPDX-License-Identifier: GPL-3.0-or-later
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("localize", Path(__file__).parents[1] / "localize.py")
localize = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(localize)

class LocalizationImportTests(unittest.TestCase):
    def test_fuzzy_and_empty_translations_are_not_imported(self):
        entries = localize.parse_po('''#, fuzzy\nmsgid "Today"\nmsgstr "Morgen"\n\nmsgid "Undo"\nmsgstr "Rückgängig"\n\nmsgid "Empty"\nmsgstr ""\n''')
        self.assertEqual(localize.confirmed_translations(entries), {"Undo": "Rückgängig"})

    def test_multiline_and_escaped_strings_survive_import(self):
        entries = localize.parse_po('msgid "Say \\"hello\\""\nmsgstr "Sag "\n"\\"Hallo\\""\n')
        self.assertEqual(localize.confirmed_translations(entries), {'Say "hello"': 'Sag "Hallo"'})

    def test_import_preserves_existing_translations(self):
        catalog = {"strings": {"Today": {"localizations": {"de": {"stringUnit": {"state": "translated", "value": "Heute"}}}}}}
        self.assertEqual(localize.import_gettext(catalog, {"Today": "Wrong"}), 0)
        self.assertEqual(catalog["strings"]["Today"]["localizations"]["de"]["stringUnit"]["value"], "Heute")

    def test_interpolation_import_reuses_the_compilers_typed_placeholder(self):
        catalog = {"strings": {"Add a task above, or press %@.": {}}}
        self.assertEqual(localize.import_gettext(catalog, {"Add a task above, or press {}.": "Füge oben eine Aufgabe hinzu oder drücke {}."}), 1)
        self.assertEqual(catalog["strings"]["Add a task above, or press %@."]["localizations"]["de"]["stringUnit"]["value"], "Füge oben eine Aufgabe hinzu oder drücke %@.")

    def test_validation_rejects_a_numeric_placeholder_changed_to_string(self):
        catalog = {"strings": {"%lld tasks deleted": {"localizations": {"de": {"stringUnit": {"state": "translated", "value": "%@ Aufgaben gelöscht"}}}}}}
        self.assertTrue(localize.validate(catalog)["errors"])

    def test_validation_accepts_numeric_plurals_and_positional_reordering(self):
        catalog = {"strings": {
            "%lld tasks deleted": {"localizations": {"de": {"variations": {"plural": {"one": {"stringUnit": {"state": "translated", "value": "%lld Aufgabe gelöscht"}}, "other": {"stringUnit": {"state": "translated", "value": "%lld Aufgaben gelöscht"}}}}}}},
            "%@ %@": {"localizations": {"de": {"stringUnit": {"state": "translated", "value": "%2$@ %1$@"}}}}
        }}
        self.assertFalse(localize.validate(catalog)["errors"])

    def test_lightweight_parser_placeholders_are_rejected(self):
        self.assertTrue(localize.validate({"strings": {"%arg tasks": {}}})["errors"])

if __name__ == "__main__":
    unittest.main()
