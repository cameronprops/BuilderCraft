"""Regression checks for permission gates on third-party UI source reuse."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_ui_reuse_manifest import validate_manifest


def record(**override):
    base = {
        "id": "test-ui", "name": "Test", "url": "https://example.org/ui",
        "license": "GPL-3.0-or-later", "license_evidence": "https://example.org/ui/LICENSE",
        "reuse_mode": "reference_only", "source_access": "source_available",
        "source_revision": None, "destinations": [], "notices": [],
        "decision": "Learning interaction pattern without copying code."
    }
    base.update(override)
    return {"schema_version": 1, "source_entries": [base]}


class UICodeProvenanceTests(unittest.TestCase):
    def test_references_are_allowed_without_copying_code(self):
        self.assertEqual([], validate_manifest(record()))

    def test_copied_gpl_source_needs_revision_destination_and_notice(self):
        x = record(reuse_mode="source_copy")
        text = "\n".join(validate_manifest(x))
        self.assertIn("pinned revision", text)
        self.assertIn("destination paths", text)
        self.assertIn("license notice", text)

    def test_copied_gpl_source_can_be_provenanced(self):
        x = record(
            reuse_mode="source_copy", source_revision="1234567890abcdef",
            destinations=["crates/ui-egui/src/borrowed_panel.rs"],
            notices=["third_party/gimp/COPYING"]
        )
        self.assertEqual([], validate_manifest(x))

    def test_free_but_closed_or_unknown_source_is_not_copyable(self):
        x = record(
            license="UNVERIFIED_PROPRIETARY", source_access="unverified",
            reuse_mode="source_copy", source_revision="1234567890",
            destinations=["crates/ui-egui/src/copy.rs"],
            notices=["notices/LICENSE"]
        )
        self.assertTrue(any("cannot be incorporated" in s for s in validate_manifest(x)))

    def test_references_cannot_claim_copied_source_files(self):
        x = record(destinations=["crates/ui-egui/src/copied.rs"])
        self.assertTrue(any("must not claim copied" in s for s in validate_manifest(x)))

    def test_duplicate_references_are_rejected(self):
        x = record()
        x["source_entries"].append(x["source_entries"][0].copy())
        self.assertTrue(any("duplicate id" in s for s in validate_manifest(x)))


if __name__ == "__main__":
    unittest.main()
