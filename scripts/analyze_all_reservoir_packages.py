import json
import time
from argparse import ArgumentParser
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import requests
from rich import print
from rich.progress import BarColumn, Progress

RESERVOIR_MANIFEST_URL = "https://reservoir.lean-lang.org/index/manifest.json"


class Args:
    manifest_file: Path
    pump_url: str
    batch_size: int
    poll_interval: int


def fetch_manifest(manifest_file: Path) -> Any:
    manifest = None
    manifest_updated = False

    try:
        manifest = json.loads(manifest_file.read_text())
        print(f"Loaded manifest from {manifest_file}")
    except FileNotFoundError:
        pass

    if manifest is None:
        print(f"Downloading manifest from {RESERVOIR_MANIFEST_URL}")
        req = requests.get(RESERVOIR_MANIFEST_URL)
        req.raise_for_status()
        manifest = req.json()
        manifest_updated = True

    if manifest_updated:
        manifest_file.write_text(json.dumps(manifest))
        print(f"Saved manifest to {manifest_file}")

    return manifest


@dataclass
class Source:
    owner: str
    repo: str


@dataclass
class Version:
    source: Source
    tag: str
    sha: str


def get_sources(manifest: Any, sources: dict[str, Source]):
    for package in manifest["packages"]:
        source = package["sources"][0]
        full_name = source["fullName"]
        owner, repo = full_name.split("/")
        sources[full_name] = Source(owner, repo)


def build_job_analyze_global(source: Source) -> dict[str, Any]:
    return {
        "version": "v0",
        "type": "analyze_global",
        "input": {
            "version": "v0",
            "source": {
                "type": "github",
                "owner": source.owner,
                "repo": source.repo,
            },
        },
    }


def build_job_analyze_version(version: Version) -> dict[str, Any]:
    return {
        "version": "v0",
        "type": "analyze_version",
        "input": {
            "version": "v0",
            "source": {
                "type": "github",
                "owner": version.source.owner,
                "repo": version.source.repo,
            },
            "sha": version.sha,
        },
    }


def process_global_info(
    sources: dict[str, Source], versions: dict[str, Version], key: str, info: Any
) -> None:
    output = info.get("output")
    if output is None:
        return

    source = sources[key]

    version_tags = (output.get("lake") or {}).get("version_tags")
    if version_tags is None:
        version_tags = output["git"]["version_tags"]

    for tag in version_tags:
        sha = output["git"]["tag_shas"][tag]
        # Flipped intentionally so queued tasks are sorted into a pseudorandom
        # but deterministic order.
        vkey = f"{sha}/{key}"
        versions[vkey] = Version(source, tag, sha)


def query(
    args: Args,
    sources: dict[str, Source],
    versions: dict[str, Version],
    info_global: dict[str, Any],
    info_version: dict[str, Any],
) -> tuple[dict[str, Any], dict[str, Any]]:
    jobs = {}

    for key, source in sorted(sources.items()):
        if len(jobs) >= args.batch_size:
            break
        if key in info_global:
            continue
        jobs[key] = build_job_analyze_global(source)

    for key, version in sorted(versions.items()):
        if len(jobs) >= args.batch_size:
            break
        if key in info_version:
            continue
        jobs[key] = build_job_analyze_version(version)

    res = requests.post(f"{args.pump_url}/query", json={"jobs": jobs})
    res.raise_for_status()
    reply = res.json()

    pending = reply["pending"]
    completed = reply["completed"]
    c_global = {k: v for k, v in completed.items() if v["type"] == "analyze_global"}
    c_version = {k: v for k, v in completed.items() if v["type"] == "analyze_version"}

    for key, result in c_global.items():
        info_global[key] = result
        process_global_info(sources, versions, key, result)

    for key, result in c_version.items():
        info_version[key] = result

    return pending, completed


def main() -> None:
    parser = ArgumentParser()
    parser.add_argument("-m", "--manifest-file", type=Path, default="manifest.json")
    parser.add_argument("-u", "--pump-url", default="http://127.0.0.1:5800")
    parser.add_argument("-b", "--batch-size", type=int, default=100)
    parser.add_argument("-i", "--poll-interval", type=int, default=2)
    args = parser.parse_args(namespace=Args())

    sources: dict[str, Source] = {}
    versions: dict[str, Version] = {}
    info_global: dict[str, Any] = {}
    info_version: dict[str, Any] = {}

    manifest = fetch_manifest(args.manifest_file)
    get_sources(manifest, sources)
    print(f"Found {len(sources)} sources in manifest")

    with Progress(
        "{task.description}",
        BarColumn(),
        "[green]{task.completed:4d}/{task.total:4d}",
    ) as p:
        tp = p.add_task("pending", total=args.batch_size)
        tr = p.add_task("running", total=args.batch_size)
        tg = p.add_task("analyze-global", total=len(sources))
        tv = p.add_task("analyze-version", total=0)
        while len(info_global) < len(sources) or len(info_version) < len(versions):
            pending, completed = query(
                args, sources, versions, info_global, info_version
            )

            running = {k: v for k, v in pending.items() if v["started"]}

            p.update(tp, completed=len(pending))
            p.update(tr, completed=len(running))
            p.update(tg, completed=len(info_global), total=len(sources))
            p.update(tv, completed=len(info_version), total=len(versions))
            p.refresh()

            if not completed:
                time.sleep(args.poll_interval)


if __name__ == "__main__":
    main()
