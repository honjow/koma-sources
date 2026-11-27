#!/usr/bin/env python3
"""Replay get_chapters through the packaged WASM with both host header shapes.

Requires Python wasmtime. No network or local/device clock is used.
"""
import argparse
from datetime import datetime
import hashlib
import json
from pathlib import Path
import struct
from urllib.parse import parse_qs, urlsplit
from zipfile import ZipFile

import wasmtime

ROOT = Path(__file__).resolve().parents[3]
DATE = "Fri, 27 Nov 2026 16:44:58 GMT"
TIMESTAMP = int(datetime.fromisoformat("2026-11-27T16:44:58+00:00").timestamp())
TOKEN = hashlib.md5(f"{TIMESTAMP}mng_ch_2026112716".encode()).hexdigest()[:16]


def replay(engine, module, headers, expected_error=None, title=""):
    store = wasmtime.Store(engine)
    requests = []

    def host(name):
        def call(caller, *args):
            if name == "log":
                return None
            if name == "check_cancel":
                return 0
            assert name == "http_request", f"Unexpected host call: {name}"
            memory = caller.get("memory")
            ptr, size, output, capacity = args
            request = json.loads(bytes(memory.read(caller, ptr, ptr + size)))
            requests.append(request)
            url = urlsplit(request["url"])
            query = parse_qs(url.query)
            assert url.scheme == "https" and url.netloc == "mangataro.org"
            assert url.path == "/auth/manga-chapters"
            assert request["method"] == "GET"
            assert request["responseMetadata"] == {"headers": True}
            assert query["manga_id"] == ["47"]
            assert query["offset"] == ["0"] and query["order"] == ["DESC"]
            if len(requests) == 1:
                # No timestamp may be guessed from a client/device clock.
                assert "_ts" not in query and "_t" not in query
                envelope = {"ok": True, "status": 403, "headers": headers,
                            "bodyText": '{"success":false}'}
            else:
                assert len(requests) == 2
                assert query["_ts"] == [str(TIMESTAMP)]
                assert query["_t"] == [TOKEN]
                body = {"success": True, "has_more": False, "chapters": [{
                    "url": "https://mangataro.org/read/one-piece/ch1-100",
                    "chapter": "1", "title": title, "language": "en"}]}
                envelope = {"ok": True, "status": 200,
                            "bodyText": json.dumps(body)}
            encoded = json.dumps(envelope, separators=(",", ":")).encode()
            assert len(encoded) <= capacity
            memory.write(caller, encoded, output)
            return len(encoded)
        return call

    imports = [wasmtime.Func(store, item.type, host(item.name), access_caller=True)
               for item in module.imports]
    exports = wasmtime.Instance(store, module, imports).exports(store)
    request = json.dumps({
        "type": "request", "version": 1, "operation": "get_chapters",
        "sourceId": "com.mangataro.en.koma", "requestId": "date-header-regression",
        "args": {"mangaId": "manga:47:one-piece"}, "settings": {},
        "hostFeatures": ["chapter_reading_order_v1"],
        "hostHints": {"network": False},
    }).encode()
    # Match koma-source-dev's request placement for SDK modules without alloc.
    ptr = (exports["koma_source_alloc"](store, len(request))
           if "koma_source_alloc" in exports else 0x10000)
    memory = exports["memory"]
    memory.write(store, request, ptr)
    result = exports["koma_source_get_chapters"](store, ptr, len(request))
    magic, _, size, _ = struct.unpack("<IIII", memory.read(store, result, result + 16))
    assert magic == 0x4B4F4D41 and 0 < size < 1024 * 1024
    response = json.loads(bytes(memory.read(store, result + 16, result + 16 + size)))
    if expected_error:
        assert response["ok"] is False, response
        assert response["error"]["code"] == expected_error, response
        assert len(requests) == 1, "Invalid Date must not trigger a clock fallback"
    else:
        assert response["ok"] is True, (response, requests)
        assert len(requests) == 2
        assert response["data"]["items"][0]["id"] == "chapter:one-piece/ch1-100"
        assert response["data"]["page"]["hasMore"] is False
        assert response["data"]["items"][0]["title"] == "Chapter 1"
    return response


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", type=Path, default=ROOT / "dist/wave-9/sources/mangataro-en/mangataro-en-0.1.0.koma")
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    with ZipFile(args.package) as package:
        wasm = package.read("source.wasm")
    engine = wasmtime.Engine()
    module = wasmtime.Module(engine, wasm)
    cases = [
        ("array", {"date": [DATE]}, None),
        ("string", {"date": DATE}, None),
        ("case-insensitive-trimmed", {"DaTe": f"  {DATE}\t"}, None),
        ("first-nonempty-array-string", {"Date": [None, "", " \t", 7, f" {DATE} ", "ignored"]}, None),
        ("missing", {}, "host_feature_required"),
        ("empty-string", {"date": " \t"}, "host_feature_required"),
        ("empty-array", {"date": []}, "host_feature_required"),
        ("invalid-array", {"date": [None, 7, {}, " "]}, "host_feature_required"),
        ("wrong-type", {"date": 123}, "host_feature_required"),
        ("missing-headers", None, "host_feature_required"),
        ("malformed-date", {"date": ["not a date", DATE]}, "parse_error"),
    ]
    results = {}
    baseline = None
    for name, headers, error in cases:
        response = replay(engine, module, headers, error)
        if error is None:
            if baseline is None:
                baseline = response["data"]
            assert response["data"] == baseline
        results[name] = {"passed": True, "result": response}
        print(f"PASS {name}")
    response = replay(engine, module, {"date": [DATE]}, title=" ")
    results["whitespace-chapter-title"] = {"passed": True, "result": response}
    print("PASS whitespace-chapter-title")
    report = {"wasmSha256": hashlib.sha256(wasm).hexdigest(),
              "fixedServerDate": DATE, "expectedTimestamp": TIMESTAMP,
              "expectedToken": TOKEN, "cases": results}
    if args.report:
        args.report.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(f"{len(results)} packaged-WASM Date header cases passed")


if __name__ == "__main__":
    main()
