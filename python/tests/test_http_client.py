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

"""End-to-end test: Python runners drive a networked sequencer over HTTP."""

import json
import subprocess
import time
from pathlib import Path

import pytest

from arc_runner.client import ProtocolError
from arc_runner.http_client import HttpArcClient
from arc_runner.identity import (
    Keypair,
    attestation_message,
    genesis_message,
    seed_validation_message,
    sign_payload,
)
from tests.test_integration import find_arc_node, hex_id

PORT = 18841


@pytest.fixture
def sequencer(tmp_path: Path):
    """Spawn an arc-node sequencer with signature enforcement."""
    arc_node_bin = find_arc_node()
    state = str(tmp_path / "state.json")
    store = tmp_path / "store"
    store.mkdir()
    authority = Keypair.from_secret_hex("31" * 32)
    key_file = tmp_path / "authority.json"
    key_file.write_text(json.dumps({"secret": authority.secret_hex}))

    subprocess.run(
        [arc_node_bin, "--state", state, "init", "--require-signatures"],
        capture_output=True, check=True,
    )
    proc = subprocess.Popen(
        [
            arc_node_bin, "--state", state, "serve",
            "--authority-key", str(key_file),
            "--listen", f"127.0.0.1:{PORT}",
            "--store", str(store),
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    client = HttpArcClient(f"http://127.0.0.1:{PORT}")
    try:
        for _ in range(50):
            try:
                client.status()
                break
            except ProtocolError:
                time.sleep(0.1)
        else:
            raise RuntimeError("sequencer did not come up")
        yield client
    finally:
        proc.kill()
        proc.wait()


def make_genesis(proposer: Keypair) -> dict:
    return {
        "id": hex_id(1),
        "rts_version": "Rts1",
        "domain_id": hex_id(1),
        "proposer": proposer.participant_id,
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
        # Funded domain: pool covers two blocks at reward 1000.
        "reward_pool": 2500,
        "validation_reserve_bps": 2000,
        "base_block_reward": 1000,
        "license_declaration": "MIT",
        "timestamp": 1700000000,
    }


class TestHttpLifecycle:
    def test_signed_lifecycle_over_http(self, sequencer: HttpArcClient):
        client = sequencer
        proposer = Keypair.from_secret_hex("41" * 32)
        validators = [Keypair.from_secret_hex(f"{50 + i:02x}" * 32) for i in range(10)]

        # Unsigned genesis refused over HTTP.
        genesis = make_genesis(proposer)
        with pytest.raises(ProtocolError, match="requires signatures"):
            client.submit_genesis(genesis)

        # Signed flow: genesis -> conformance -> seed validations ->
        # activation -> validator registration.
        signed = sign_payload(genesis, genesis_message(genesis), proposer)
        assert client.submit_genesis(signed)["genesis_id"] == hex_id(1)
        client.evaluate_conformance(hex_id(1))
        for v in validators[:3]:
            record = {
                "validator": v.participant_id,
                "vote": "Pass",
                "observed_score": 0.93,
                "timestamp": 1700000001,
            }
            msg = seed_validation_message(hex_id(1), record)
            client.record_seed_validation(
                hex_id(1), sign_payload(record, msg, v)
            )
        client.finalize_activation(hex_id(1))
        client.register_validators({
            "domain_id": hex_id(1),
            "validators": [v.participant_id for v in validators],
        })

        # Funded-domain pool is queryable over HTTP.
        pool = client.show_pool(hex_id(1))
        assert pool["balance"] == 2000
        assert pool["dormant"] is False

        # Evidence artifact travels through the sequencer store.
        evidence = json.dumps({"metrics": {"score": 0.95}}).encode()
        evidence_hash = client.put_artifact(evidence)
        assert client.get_artifact(evidence_hash) == evidence

        # Signed block submission and validation.
        block = {
            "id": hex_id(60),
            "domain_id": hex_id(1),
            "parent_id": hex_id(1),
            "proposer": proposer.participant_id,
            "child_state_ref": hex_id(61),
            "diff_ref": hex_id(62),
            "claimed_metric_delta": 0.015,
            "evidence_bundle_hash": evidence_hash,
            "fee": 10,
            "bond": 500,
            "epoch_id": 1,
            "status": "Submitted",
            "timestamp": 1700001000,
        }
        from arc_runner.identity import block_message

        client.submit_block(sign_payload(block, block_message(block), proposer))
        assigned = client.assign_validators(hex_id(60))["assigned_validators"]
        assert len(assigned) == 3
        by_id = {v.participant_id: v for v in validators}
        for validator_hex in assigned:
            att = {
                "block_id": hex_id(60),
                "validator": validator_hex,
                "vote": "Pass",
                "observed_delta": 0.015,
                "replay_evidence_ref": hex_id(70),
                "timestamp": 1700002000,
            }
            signed_att = sign_payload(
                att, attestation_message(att), by_id[validator_hex]
            )
            client.submit_attestation(signed_att)
        assert client.evaluate_block(hex_id(60))["outcome"] == "Accepted"

        # Frontier + block detail over HTTP; pool debited.
        assert client.show_frontier(hex_id(1))["canonical_frontier"] == hex_id(60)
        detail = client.show_block(hex_id(60))
        assert detail["derived_validity"] == "DirectValid"
        assert client.show_pool(hex_id(1))["spent"] == 1000

        # Settle and confirm ordering advanced.
        client.close_challenge_window(hex_id(60))
        for _ in range(5):
            client.advance_epoch()
        client.settle_block(hex_id(60))
        status = client.status()
        assert status["seq"] > 10
        assert status["require_signatures"] is True
