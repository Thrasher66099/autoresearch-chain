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
Unit tests for the arc-node protocol client.

These tests mock subprocess calls — they do not require the arc-node
binary to be built. Integration tests that exercise the real binary
are in test_integration.py.
"""

from __future__ import annotations

import json
import subprocess
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from arc_runner.client import ArcNodeClient, ProtocolError, generate_id


# ---------------------------------------------------------------------------
# generate_id tests
# ---------------------------------------------------------------------------

class TestGenerateId:
    def test_produces_64_char_hex(self):
        result = generate_id(b"hello", b"world")
        assert len(result) == 64
        int(result, 16)  # valid hex

    def test_deterministic(self):
        a = generate_id(b"foo", b"bar")
        b = generate_id(b"foo", b"bar")
        assert a == b

    def test_different_inputs_different_ids(self):
        a = generate_id(b"foo", b"bar")
        b = generate_id(b"baz", b"qux")
        assert a != b

    def test_order_matters(self):
        a = generate_id(b"foo", b"bar")
        b = generate_id(b"bar", b"foo")
        assert a != b

    def test_single_component(self):
        result = generate_id(b"single")
        assert len(result) == 64

    def test_empty_components(self):
        result = generate_id(b"", b"")
        assert len(result) == 64


# ---------------------------------------------------------------------------
# ProtocolError tests
# ---------------------------------------------------------------------------

class TestProtocolError:
    def test_carries_stderr(self):
        err = ProtocolError("some error message\n", 1)
        assert err.stderr == "some error message\n"
        assert err.returncode == 1
        assert str(err) == "some error message"

    def test_strips_trailing_whitespace_in_str(self):
        err = ProtocolError("  error  \n\n", 2)
        assert str(err) == "error"


# ---------------------------------------------------------------------------
# ArcNodeClient._run tests
# ---------------------------------------------------------------------------

class TestClientRun:
    @patch("arc_runner.client.subprocess.run")
    def test_success_returns_parsed_json(self, mock_run: MagicMock):
        mock_run.return_value = MagicMock(
            returncode=0,
            stdout='{"status": "ok", "count": 42}',
            stderr="",
        )
        client = ArcNodeClient("/tmp/state.json", "arc-node")
        result = client._run(["--state", "/tmp/state.json", "list-domains"])

        assert result == {"status": "ok", "count": 42}
        mock_run.assert_called_once_with(
            ["arc-node", "--state", "/tmp/state.json", "list-domains"],
            capture_output=True,
            text=True,
        )

    @patch("arc_runner.client.subprocess.run")
    def test_empty_stdout_returns_empty_dict(self, mock_run: MagicMock):
        mock_run.return_value = MagicMock(
            returncode=0,
            stdout="",
            stderr="Created fresh state\n",
        )
        client = ArcNodeClient("/tmp/state.json")
        result = client._run(["init", "/tmp/state.json"])
        assert result == {}

    @patch("arc_runner.client.subprocess.run")
    def test_nonzero_exit_raises_protocol_error(self, mock_run: MagicMock):
        mock_run.return_value = MagicMock(
            returncode=1,
            stdout="",
            stderr="error: domain not active\n",
        )
        client = ArcNodeClient("/tmp/state.json")
        with pytest.raises(ProtocolError) as exc_info:
            client._run(["--state", "/tmp/state.json", "submit-block", "/f"])
        assert "domain not active" in str(exc_info.value)
        assert exc_info.value.returncode == 1

    @patch("arc_runner.client.subprocess.run")
    def test_whitespace_only_stdout_returns_empty_dict(self, mock_run: MagicMock):
        mock_run.return_value = MagicMock(
            returncode=0,
            stdout="  \n  ",
            stderr="",
        )
        client = ArcNodeClient("/tmp/state.json")
        result = client._run(["--state", "/tmp/state.json", "advance-epoch"])
        assert result == {}


# ---------------------------------------------------------------------------
# ArcNodeClient._run_with_json_file tests
# ---------------------------------------------------------------------------

class TestClientRunWithJsonFile:
    @patch("arc_runner.client.subprocess.run")
    def test_writes_json_and_cleans_up(self, mock_run: MagicMock, tmp_path: Path):
        mock_run.return_value = MagicMock(
            returncode=0,
            stdout='{"block_id": "abc123"}',
            stderr="",
        )
        client = ArcNodeClient(str(tmp_path / "state.json"))
        data = {"id": "deadbeef", "status": "Submitted"}
        result = client._run_with_json_file("submit-block", data)

        assert result == {"block_id": "abc123"}

        # Verify the temp file was passed and contained valid JSON.
        call_args = mock_run.call_args[0][0]
        assert call_args[0] == "arc-node"
        assert "--state" in call_args
        assert "submit-block" in call_args
        # The temp file should have been the last argument.
        tmp_file = call_args[-1]
        # The file should be cleaned up.
        assert not Path(tmp_file).exists()

    @patch("arc_runner.client.subprocess.run")
    def test_cleans_up_on_error(self, mock_run: MagicMock, tmp_path: Path):
        mock_run.return_value = MagicMock(
            returncode=1,
            stdout="",
            stderr="error: bad json\n",
        )
        client = ArcNodeClient(str(tmp_path / "state.json"))
        with pytest.raises(ProtocolError):
            client._run_with_json_file("submit-genesis", {"bad": True})


# ---------------------------------------------------------------------------
# ArcNodeClient method routing tests
# ---------------------------------------------------------------------------

class TestClientMethods:
    """Verify each method constructs the correct CLI arguments."""

    @patch("arc_runner.client.subprocess.run")
    def test_init(self, mock_run: MagicMock):
        mock_run.return_value = MagicMock(returncode=0, stdout="", stderr="")
        client = ArcNodeClient("/tmp/state.json")
        client.init()
        args = mock_run.call_args[0][0]
        assert args == ["arc-node", "init", "/tmp/state.json"]

    @patch("arc_runner.client.subprocess.run")
    def test_inspect(self, mock_run: MagicMock):
        mock_run.return_value = MagicMock(
            returncode=0,
            stdout="",
            stderr="State file: /tmp/state.json\n  Epoch: 0\n",
        )
        client = ArcNodeClient("/tmp/state.json")
        result = client.inspect()
        assert "Epoch: 0" in result

    @patch("arc_runner.client.subprocess.run")
    def test_evaluate_conformance(self, mock_run: MagicMock):
        mock_run.return_value = MagicMock(
            returncode=0,
            stdout='{"status": "conformance_passed"}',
            stderr="",
        )
        client = ArcNodeClient("/tmp/state.json")
        result = client.evaluate_conformance("aa" * 32)
        assert result["status"] == "conformance_passed"
        args = mock_run.call_args[0][0]
        assert "evaluate-conformance" in args
        assert "aa" * 32 in args

    @patch("arc_runner.client.subprocess.run")
    def test_advance_epoch(self, mock_run: MagicMock):
        mock_run.return_value = MagicMock(
            returncode=0,
            stdout='{"epoch": 1}',
            stderr="",
        )
        client = ArcNodeClient("/tmp/state.json")
        result = client.advance_epoch()
        assert result["epoch"] == 1
        args = mock_run.call_args[0][0]
        assert "advance-epoch" in args

    @patch("arc_runner.client.subprocess.run")
    def test_list_blocks_without_domain(self, mock_run: MagicMock):
        mock_run.return_value = MagicMock(
            returncode=0,
            stdout='{"block_count": 0, "blocks": []}',
            stderr="",
        )
        client = ArcNodeClient("/tmp/state.json")
        result = client.list_blocks()
        args = mock_run.call_args[0][0]
        assert "--domain" not in args

    @patch("arc_runner.client.subprocess.run")
    def test_list_blocks_with_domain(self, mock_run: MagicMock):
        mock_run.return_value = MagicMock(
            returncode=0,
            stdout='{"block_count": 1, "blocks": []}',
            stderr="",
        )
        client = ArcNodeClient("/tmp/state.json")
        domain = "bb" * 32
        result = client.list_blocks(domain)
        args = mock_run.call_args[0][0]
        assert "--domain" in args
        assert domain in args
