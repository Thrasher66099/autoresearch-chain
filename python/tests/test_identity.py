# Copyright (C) 2026 AutoResearch Chain contributors
#
# This file is part of AutoResearch Chain.
#
# AutoResearch Chain is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# AutoResearch Chain is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
# See the GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.

"""Ed25519 identity tests, including cross-language node verification."""

import json
from pathlib import Path

from arc_runner.identity import (
    Keypair,
    attestation_message,
    genesis_message,
    sign_payload,
)
from tests.test_integration import find_arc_node, hex_id


class TestMessageFormat:
    def test_attestation_message_matches_rust_pinned_vector(self):
        # Must match the pinned vector in crates/identity tests.
        att = {
            "block_id": "b1",
            "validator": "v1",
            "vote": "Pass",
            "observed_delta": 0.015,
            "replay_evidence_ref": "r1",
            "timestamp": 5,
        }
        assert (
            attestation_message(att)
            == b"arc-attestation-v1|b1|v1|Pass|3f8eb851eb851eb8|r1|5"
        )
        att["vote"] = "Fail"
        del att["observed_delta"]
        assert (
            attestation_message(att)
            == b"arc-attestation-v1|b1|v1|Fail|none|r1|5"
        )

    def test_keypair_identity_is_public_key(self):
        kp = Keypair.from_secret_hex("07" * 32)
        assert len(kp.participant_id) == 64
        # Deterministic: same secret, same identity.
        assert Keypair.from_secret_hex("07" * 32).participant_id == kp.participant_id


class TestCrossLanguageVerification:
    """Python signs; the Rust node verifies (require-signatures state)."""

    def test_signed_genesis_accepted_unsigned_rejected(self, tmp_path: Path):
        arc_node_bin = find_arc_node()
        import subprocess

        state = str(tmp_path / "state.json")
        subprocess.run(
            [arc_node_bin, "--state", state, "init", "--require-signatures"],
            capture_output=True, check=True,
        )

        kp = Keypair.from_secret_hex("11" * 32)
        genesis = {
            "id": hex_id(1),
            "rts_version": "Rts1",
            "domain_id": hex_id(1),
            "proposer": kp.participant_id,
            "research_target_declaration": (
                "Improve CIFAR-10 training recipe accuracy within fixed "
                "compute budget"
            ),
            "domain_intent": "EndToEndRecipeImprovement",
            "seed_recipe_ref": hex_id(10),
            "seed_codebase_state_ref": hex_id(11),
            "frozen_surface": ["eval/"],
            "search_surface": ["train.py"],
            "canonical_dataset_ref": hex_id(20),
            "dataset_hash": hex_id(21),
            "dataset_splits": {
                "training": hex_id(22),
                "validation": hex_id(23),
                "test": hex_id(24),
            },
            "evaluation_harness_ref": hex_id(30),
            "metric_id": "test_accuracy",
            "metric_direction": "HigherBetter",
            "hardware_class": "RTX 4090",
            "time_budget_secs": 3600,
            "seed_environment_manifest_ref": hex_id(40),
            "seed_score": 0.93,
            "artifact_schema_ref": hex_id(50),
            "seed_bond": 1000,
            "license_declaration": "MIT",
            "timestamp": 1700000000,
        }

        # Unsigned: refused.
        unsigned = tmp_path / "genesis_unsigned.json"
        unsigned.write_text(json.dumps(genesis))
        proc = subprocess.run(
            [arc_node_bin, "--state", state, "submit-genesis", str(unsigned)],
            capture_output=True, text=True,
        )
        assert proc.returncode != 0
        assert "requires signatures" in proc.stderr

        # Signed in Python, verified by the Rust node: accepted.
        signed = sign_payload(genesis, genesis_message(genesis), kp)
        signed_file = tmp_path / "genesis_signed.json"
        signed_file.write_text(json.dumps(signed))
        proc = subprocess.run(
            [arc_node_bin, "--state", state, "submit-genesis", str(signed_file)],
            capture_output=True, text=True,
        )
        assert proc.returncode == 0, proc.stderr
        assert json.loads(proc.stdout)["genesis_id"] == hex_id(1)

        # Tampered after signing: rejected.
        tampered = dict(signed)
        tampered["id"] = hex_id(2)
        tampered["domain_id"] = hex_id(2)
        tampered_file = tmp_path / "genesis_tampered.json"
        tampered_file.write_text(json.dumps(tampered))
        proc = subprocess.run(
            [arc_node_bin, "--state", state, "submit-genesis", str(tampered_file)],
            capture_output=True, text=True,
        )
        assert proc.returncode != 0
        assert "signature rejected" in proc.stderr
