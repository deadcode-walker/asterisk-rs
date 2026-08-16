#!/usr/bin/env python3
"""Check the pinned Asterisk contract and the deliberately supported local surface."""

from __future__ import annotations

import json
import hashlib
import re
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "docs/references/asterisk-22.9.0.json"
UPSTREAM_INVENTORY = ROOT / "docs/references/asterisk-22.9.0-inventory.json"
LOCAL_INVENTORY = ROOT / "docs/references/asterisk-22.9.0-local.json"


def fetch(url: str) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": "asterisk-rs-contract-check"})
    with urllib.request.urlopen(request, timeout=30) as response:
        return response.read()


def c_function_body(source: str, name: str) -> str:
    signature = re.search(rf"static [^\n]*\b{re.escape(name)}\s*\([^)]*\)\s*\{{", source)
    if signature is None:
        raise SystemExit(f"cannot locate pinned media function {name}")
    start = source.find("{", signature.start())
    depth = 0
    for end in range(start, len(source)):
        depth += (source[end] == "{") - (source[end] == "}")
        if depth == 0:
            return source[start : end + 1]
    raise SystemExit(f"unterminated pinned media function {name}")


def upstream_media_fields(
    media: bytes, contract: dict[str, object]
) -> tuple[dict[str, list[str]], dict[str, list[str]]]:
    source = media.decode("utf-8")
    event_fields: dict[str, list[str]] = {}
    for wire_name in contract["local"]["media_events"]:
        macro = re.search(
            rf"#define\s+_create_event_{re.escape(wire_name)}\([^\n]+_create_event_nodata",
            source,
        )
        if macro is not None:
            event_fields[wire_name] = ["channel_id"]
            continue
        body = c_function_body(source, f"_create_event_{wire_name}")
        json_pack = re.search(r"ast_json_pack\((.*?)\);", body, re.DOTALL)
        if json_pack is None:
            raise SystemExit(f"pinned media event {wire_name} has no JSON contract")
        keys = re.findall(r'^\s*"([a-z][a-z0-9_]*)"\s*,', json_pack.group(1), re.MULTILINE)
        event_fields[wire_name] = sorted(key for key in keys if key != "event")

    defines = dict(re.findall(r'^#define\s+([A-Z0-9_]+)\s+"([A-Z_]+)"', source, re.MULTILINE))
    command_fields: dict[str, list[str]] = {}
    handle = c_function_body(source, "handle_command")
    for wire_name in contract["local"]["media_commands"]:
        constants = [name for name, value in defines.items() if value == wire_name]
        if len(constants) != 1:
            raise SystemExit(f"cannot map pinned media command {wire_name} to one constant")
        branch = re.search(
            rf"ast_strings_equal\(command,\s*{re.escape(constants[0])}\)\)(.*?)(?=\n\s*\}} else if|\n\s*\}} else \{{)",
            handle,
            re.DOTALL,
        )
        if branch is None:
            raise SystemExit(f"cannot locate pinned media command branch {wire_name}")
        command_fields[wire_name] = sorted(
            set(re.findall(r'ast_json_object_string_get\(json,\s*"([a-z_]+)"\)', branch.group(1)))
        )
    return event_fields, command_fields


def verify_upstream(contract: dict[str, object], upstream: dict[str, object]) -> None:
    source = contract["source"]
    commit = source["commit"]
    base = f"https://raw.githubusercontent.com/asterisk/asterisk/{commit}"
    generated_routes: list[dict[str, str]] = []
    generated_models: set[str] = set()

    for name, expected_digest in source["ari_documents"].items():
        content = fetch(f"{base}/rest-api/api-docs/{name}")
        actual_digest = hashlib.sha256(content).hexdigest()
        if actual_digest != expected_digest:
            raise SystemExit(
                f"upstream digest mismatch for {name}: expected {expected_digest}, got {actual_digest}"
            )
        document = json.loads(content)
        for api in document.get("apis", []):
            for operation in api.get("operations", []):
                generated_routes.append(
                    {
                        "method": operation["httpMethod"],
                        "path": api["path"],
                        "operation": operation["nickname"],
                    }
                )
        generated_models.update(document.get("models", {}).keys())

    media = fetch(f"{base}/channels/chan_websocket.c")
    media_digest = hashlib.sha256(media).hexdigest()
    if media_digest != source["chan_websocket_sha256"]:
        raise SystemExit(
            "upstream digest mismatch for channels/chan_websocket.c: "
            f"expected {source['chan_websocket_sha256']}, got {media_digest}"
        )

    event_fields, command_fields = upstream_media_fields(media, contract)
    if event_fields != contract["local"]["media_event_fields"]:
        raise SystemExit(
            f"pinned media event fields drift: expected {contract['local']['media_event_fields']}, "
            f"got {event_fields}"
        )
    if command_fields != contract["local"]["media_command_fields"]:
        raise SystemExit(
            f"pinned media command fields drift: expected {contract['local']['media_command_fields']}, "
            f"got {command_fields}"
        )

    generated = {
        "upstream_routes": sorted(
            generated_routes,
            key=lambda route: (route["path"], route["method"], route["operation"]),
        ),
        "upstream_models": sorted(generated_models),
    }
    if generated != upstream:
        raise SystemExit("checked-in upstream inventory does not match the pinned Asterisk source")

    print("pinned Asterisk source digests and generated inventory are valid")


def explicit_wire_names(source: str, enum_name: str) -> set[str]:
    variants = set(
        re.findall(r"^\s*([A-Z][A-Za-z0-9_]*)\s*(?:\{|,)", source, re.MULTILINE)
    )
    renamed = re.findall(
        r'#\[serde\(rename\s*=\s*"([A-Z_]+)"\)\]\s*([A-Z][A-Za-z0-9_]*)',
        source,
    )
    renamed_variants = {variant for _, variant in renamed}
    wire_names = {wire_name for wire_name, _ in renamed}
    if variants != renamed_variants or len(wire_names) != len(renamed):
        raise SystemExit(
            f"{enum_name} variants require unique explicit wire names: "
            f"variants={sorted(variants)}, renamed={sorted(renamed_variants)}"
        )
    return wire_names


def explicit_wire_fields(source: str, enum_name: str) -> dict[str, list[str]]:
    contracts: dict[str, list[str]] = {}
    pattern = re.compile(
        r'#\[serde\(rename\s*=\s*"([A-Z_]+)"\)\]\s*'
        r'[A-Z][A-Za-z0-9_]*\s*(?:\{(.*?)\}|,)',
        re.DOTALL,
    )
    for wire_name, body in pattern.findall(source):
        fields = re.findall(r"\b([a-z][a-z0-9_]*)\s*:", body)
        contracts[wire_name] = sorted(fields)
    if len(contracts) != len(pattern.findall(source)):
        raise SystemExit(f"{enum_name} wire field names must be unique")
    return contracts


def enum_source(source: str, name: str, next_name: str) -> str:
    match = re.search(
        rf"pub enum {re.escape(name)} \{{(.*?)(?:pub )?enum {re.escape(next_name)} \{{",
        source,
        re.DOTALL,
    )
    if match is None:
        raise SystemExit(f"cannot locate {name} in media source")
    return match.group(1)


def canonical_path(path: str) -> str:
    path = re.sub(r"\{[^}]*\}", "{}", path)
    if path.endswith("{}") and not path.endswith("/{}"):
        path = path[:-2]
    return path


def rust_function_bodies(source: str) -> list[tuple[str, str]]:
    functions = []
    for match in re.finditer(r"^\s*pub async fn ([A-Za-z0-9_]+)", source, re.MULTILINE):
        start = source.find("{", match.end())
        if start == -1:
            raise SystemExit(f"cannot locate body for ARI function {match.group(1)}")
        depth = 0
        for end in range(start, len(source)):
            depth += (source[end] == "{") - (source[end] == "}")
            if depth == 0:
                functions.append((match.group(1), source[start : end + 1]))
                break
        else:
            raise SystemExit(f"unterminated body for ARI function {match.group(1)}")
    return functions


def extract_local_routes(upstream_routes: list[dict[str, str]]) -> list[dict[str, str]]:
    method_names = {
        "get": "GET",
        "post": "POST",
        "post_empty": "POST",
        "put": "PUT",
        "put_empty": "PUT",
        "delete": "DELETE",
        "delete_with_response": "DELETE",
    }
    routes: dict[tuple[str, str, str], dict[str, str]] = {}
    resource_dir = ROOT / "crates/asterisk-rs-ari/src/resources"
    for path in resource_dir.glob("*.rs"):
        if path.stem == "mod":
            continue
        for function, body in rust_function_bodies(path.read_text(encoding="utf-8")):
            methods = {
                method_names[name]
                for name in re.findall(
                    r"\.(get|post|post_empty|put|put_empty|delete|delete_with_response)\s*\(",
                    body,
                )
            }
            paths = {
                canonical_path(value)
                for value in re.findall(r'"(/[^"?\n]*)', body)
            }
            if len(methods) != 1 or len(paths) != 1:
                raise SystemExit(
                    f"cannot infer one ARI route for {path.stem}::{function}: "
                    f"methods={sorted(methods)}, paths={sorted(paths)}"
                )
            method = methods.pop()
            local_path = paths.pop()
            matches = [
                route
                for route in upstream_routes
                if route["method"] == method
                and canonical_path(route["path"]) == local_path
            ]
            if len(matches) != 1:
                raise SystemExit(
                    f"local ARI route {path.stem}::{function} does not map uniquely upstream: "
                    f"{method} {local_path}"
                )
            upstream = matches[0]
            entry = {
                "symbol": f"{path.stem}::{function}",
                "method": method,
                "path": upstream["path"],
                "operation": upstream["operation"],
            }
            routes[(entry["symbol"], method, entry["path"])] = entry
    return sorted(routes.values(), key=lambda route: route["symbol"])


def main() -> None:
    contract = json.loads(MANIFEST.read_text(encoding="utf-8"))
    upstream = json.loads(UPSTREAM_INVENTORY.read_text(encoding="utf-8"))
    local = json.loads(LOCAL_INVENTORY.read_text(encoding="utf-8"))
    source = contract["source"]
    if not re.fullmatch(r"[0-9a-f]{40}", source["commit"]) or not re.fullmatch(
        r"[0-9a-f]{64}", source["chan_websocket_sha256"]
    ):
        raise SystemExit("pinned Asterisk identity is malformed")
    if len(source["ari_documents"]) != 11:
        raise SystemExit("pinned ARI document set must contain all eleven resource documents")
    if any(
        re.fullmatch(r"[0-9a-f]{64}", digest) is None
        for digest in source["ari_documents"].values()
    ):
        raise SystemExit("pinned ARI document digest is malformed")

    if "--verify-upstream" in sys.argv[1:]:
        verify_upstream(contract, upstream)
    elif sys.argv[1:]:
        raise SystemExit(f"unknown argument: {sys.argv[1]}")

    routes = upstream["upstream_routes"]
    route_keys = {(route["method"], route["path"], route["operation"]) for route in routes}
    models = upstream["upstream_models"]
    if len(routes) != source["ari_operation_count"] or len(route_keys) != len(routes):
        raise SystemExit("generated upstream ARI route inventory is incomplete or duplicated")
    if len(models) != source["ari_model_count"] or len(set(models)) != len(models):
        raise SystemExit("generated upstream ARI model inventory is incomplete or duplicated")

    resources = {
        path.stem
        for path in (ROOT / "crates/asterisk-rs-ari/src/resources").glob("*.rs")
        if path.stem != "mod"
    }
    expected_resources = set(contract["local"]["resource_modules"])
    if resources != expected_resources:
        raise SystemExit(
            f"ARI resource coverage drift: expected {sorted(expected_resources)}, got {sorted(resources)}"
        )

    model_symbols: set[str] = set()
    for path in (ROOT / "crates/asterisk-rs-ari/src/resources").glob("*.rs"):
        if path.stem == "mod":
            continue
        text = path.read_text(encoding="utf-8")
        model_symbols.update(
            f"{path.stem}::{name}"
            for name in re.findall(r"^pub (?:struct|enum) ([A-Za-z0-9_]+)", text, re.MULTILINE)
        )
    event_source = (ROOT / "crates/asterisk-rs-ari/src/event.rs").read_text(encoding="utf-8")
    model_symbols.update(
        f"event::{name}"
        for name in re.findall(
            r"^pub (?:struct|enum) ([A-Za-z0-9_]+)", event_source, re.MULTILINE
        )
    )
    actual_routes = extract_local_routes(routes)
    expected_routes = local["local_routes"]
    expected_models = set(local["local_model_symbols"])
    if actual_routes != expected_routes:
        raise SystemExit(
            f"local ARI route inventory drift: expected {expected_routes}, got {actual_routes}"
        )
    if model_symbols != expected_models:
        raise SystemExit(
            f"local ARI model inventory drift: expected {sorted(expected_models)}, got {sorted(model_symbols)}"
        )

    media = (ROOT / "crates/asterisk-rs-ari/src/media.rs").read_text(encoding="utf-8")
    local_names = {
        "media_events": explicit_wire_names(
            enum_source(media, "MediaEvent", "MediaDirection"), "MediaEvent"
        ),
        "media_commands": explicit_wire_names(
            enum_source(media, "MediaCommand", "InternalCmd"), "MediaCommand"
        ),
    }
    for category, actual in local_names.items():
        expected = set(contract["local"][category])
        if actual != expected:
            raise SystemExit(
                f"{category} drift: expected {sorted(expected)}, got {sorted(actual)}"
            )

    local_fields = {
        "media_event_fields": explicit_wire_fields(
            enum_source(media, "MediaEvent", "MediaDirection"), "MediaEvent"
        ),
        "media_command_fields": explicit_wire_fields(
            enum_source(media, "MediaCommand", "InternalCmd"), "MediaCommand"
        ),
    }
    for category, actual in local_fields.items():
        expected = contract["local"][category]
        if actual != expected:
            raise SystemExit(f"{category} drift: expected {expected}, got {actual}")

    print("pinned Asterisk 22.9.0 contract and local coverage are valid")


if __name__ == "__main__":
    main()
