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

"""
Ed25519 identity for AutoResearch Chain runners (Milestone E1).

A participant's identity **is** their Ed25519 public key: the 64-char hex
participant/validator/proposer ID on protocol payloads is the raw public
key. Signing messages are deterministic, domain-separated strings that
must match ``crates/identity`` byte-for-byte: fields joined with ``|``
after a versioned type tag; IDs as 64-char lowercase hex; integers in
decimal; floats as the big-endian hex of their IEEE-754 f64 bits; absent
optionals as the literal ``none``.

Payloads carry the signature as a sibling ``signature`` field (128 hex
chars), verified by ``arc-node`` against the actor field.
"""

from __future__ import annotations

import struct

from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
)


def _f64_bits_hex(x: float) -> str:
    return struct.pack(">d", float(x)).hex()


def _opt_f64(x: float | None) -> str:
    return "none" if x is None else _f64_bits_hex(x)


class Keypair:
    """An Ed25519 keypair; the public key doubles as the participant ID."""

    def __init__(self, private: Ed25519PrivateKey) -> None:
        self._private = private

    @classmethod
    def generate(cls) -> "Keypair":
        return cls(Ed25519PrivateKey.generate())

    @classmethod
    def from_secret_hex(cls, secret_hex: str) -> "Keypair":
        return cls(
            Ed25519PrivateKey.from_private_bytes(bytes.fromhex(secret_hex))
        )

    @property
    def secret_hex(self) -> str:
        from cryptography.hazmat.primitives.serialization import (
            Encoding,
            NoEncryption,
            PrivateFormat,
        )

        return self._private.private_bytes(
            Encoding.Raw, PrivateFormat.Raw, NoEncryption()
        ).hex()

    @property
    def participant_id(self) -> str:
        """64-char hex public key — the on-protocol identity."""
        from cryptography.hazmat.primitives.serialization import (
            Encoding,
            PublicFormat,
        )

        return (
            self._private.public_key()
            .public_bytes(Encoding.Raw, PublicFormat.Raw)
            .hex()
        )

    def sign(self, message: bytes) -> str:
        """Sign a message, returning the 128-char hex signature."""
        return self._private.sign(message).hex()


def block_message(block: dict) -> bytes:
    """Signing message for a block payload (mirrors Rust block_message)."""
    return (
        "arc-block-v1|{id}|{domain_id}|{parent_id}|{proposer}|"
        "{child_state_ref}|{diff_ref}|{delta}|{evidence_bundle_hash}|"
        "{fee}|{bond}|{epoch_id}|{timestamp}".format(
            id=block["id"],
            domain_id=block["domain_id"],
            parent_id=block["parent_id"],
            proposer=block["proposer"],
            child_state_ref=block["child_state_ref"],
            diff_ref=block["diff_ref"],
            delta=_f64_bits_hex(block["claimed_metric_delta"]),
            evidence_bundle_hash=block["evidence_bundle_hash"],
            fee=block["fee"],
            bond=block["bond"],
            epoch_id=block["epoch_id"],
            timestamp=block["timestamp"],
        )
    ).encode("utf-8")


def attestation_message(att: dict) -> bytes:
    """Signing message for an attestation payload."""
    return (
        f"arc-attestation-v1|{att['block_id']}|{att['validator']}|"
        f"{att['vote']}|{_opt_f64(att.get('observed_delta'))}|"
        f"{att['replay_evidence_ref']}|{att['timestamp']}"
    ).encode("utf-8")


def genesis_message(genesis: dict) -> bytes:
    """Signing message for a genesis payload (the ID commits to content)."""
    return (
        f"arc-genesis-v1|{genesis['id']}|{genesis['proposer']}|"
        f"{genesis['timestamp']}"
    ).encode("utf-8")


def challenge_message(params: dict) -> bytes:
    """Signing message for an open-challenge payload."""
    target = params["target"]
    target_block_id = next(iter(target.values()))["block_id"]
    return (
        f"arc-challenge-v1|{params['challenge_id']}|"
        f"{params['challenge_type']}|{target_block_id}|"
        f"{params['challenger']}|{params['bond']}|{params['evidence_ref']}"
    ).encode("utf-8")


def seed_validation_message(genesis_id: str, record: dict) -> bytes:
    """Signing message for a seed validation record."""
    return (
        f"arc-seedval-v1|{genesis_id}|{record['validator']}|"
        f"{record['vote']}|{_opt_f64(record.get('observed_score'))}|"
        f"{record['timestamp']}"
    ).encode("utf-8")


def sign_payload(payload: dict, message: bytes, keypair: Keypair) -> dict:
    """Return a copy of the payload with the signature field attached."""
    signed = dict(payload)
    signed["signature"] = keypair.sign(message)
    return signed
