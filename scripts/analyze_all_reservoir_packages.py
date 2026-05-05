import json
import time
from argparse import ArgumentParser
from pathlib import Path
from typing import Any

import requests
from rich import print
from rich.progress import BarColumn, Progress
from semver import Version

RESERVOIR_MANIFEST_URL = "https://reservoir.lean-lang.org/index/manifest.json"


class Args:
    manifest_file: Path
    pump_url: str
    batch_size: int
    poll_interval: int
    basic_username: str
    basic_password: str
    skip_version: bool
    skip_build: bool


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


def get_sources_from_manifest(manifest: Any) -> list[tuple[str, str]]:
    result = []
    for package in manifest["packages"]:
        source = package["sources"][0]
        full_name = source["fullName"]
        owner, repo = full_name.split("/")
        result.append((owner, repo))
    return result


class Collector:
    def __init__(self):
        self.queries: dict[str, Any] = {}
        self.results: dict[str, Any] = {}

    def query(self, key: str, job: Any) -> Any | None:
        self.queries[key] = job
        return self.results.get(key)

    def _open_queries(self, limit: int) -> dict[str, Any]:
        result = {}
        for key, job in sorted(self.queries.items()):
            if len(result) >= limit:
                break
            if key in self.results:
                continue
            result[key] = job
        return result

    def _update_results(self, new_results: dict[str, Any]) -> None:
        self.results.update(new_results)

    def update(
        self, pump_url: str, auth: tuple[str, str], limit: int
    ) -> tuple[dict[str, Any], dict[str, Any]]:
        jobs = self._open_queries(limit)

        res = requests.post(f"{pump_url}/query", json={"jobs": jobs}, auth=auth)
        res.raise_for_status()
        reply = res.json()
        pending = reply["pending"]
        completed = reply["completed"]

        self._update_results(completed)
        return pending, completed

    def status_prefixed(self, prefix: str) -> tuple[int, int]:
        queries = sum(1 for k in self.queries if k.startswith(prefix))
        results = sum(1 for k in self.results if k.startswith(prefix))
        return queries, results


def parse_version(version: str) -> Version | None:
    try:
        return Version.parse(version.removeprefix("v"))
    except ValueError:
        pass


def sorted_versions(versions: list[str]) -> list[str]:
    pv = [(p, v) for v in versions if (p := parse_version(v))]
    pv.sort()
    return [v for _, v in pv]


def derive_jobs(
    collector: Collector,
    sources: list[tuple[str, str]],
    skip_version: bool,
    skip_build: bool,
) -> None:
    for owner, repo in sorted(sources):
        result_global = collector.query(
            f"a/{owner}/{repo}",
            {
                "version": "v0",
                "type": "analyze_global",
                "input": {
                    "version": "v0",
                    "source": {"type": "github", "owner": owner, "repo": repo},
                },
            },
        )
        if result_global is None:
            continue
        output_global = result_global.get("output")
        if output_global is None:
            continue

        if skip_version:
            continue

        version_tags = (output_global.get("lake") or {}).get("version_tags")
        if version_tags is None:
            version_tags = output_global["git"]["version_tags"]

        for tag in version_tags:
            sha = output_global["git"]["tag_shas"][tag]
            collector.query(
                f"b/{sha}/{owner}/{repo}",
                {
                    "version": "v0",
                    "type": "analyze_version",
                    "input": {
                        "version": "v0",
                        "source": {"type": "github", "owner": owner, "repo": repo},
                        "sha": sha,
                    },
                },
            )

        if skip_build:
            continue

        sorted_version_tags = sorted_versions(version_tags)
        if sorted_version_tags:
            tag = sorted_version_tags[-1]
            sha = output_global["git"]["tag_shas"][tag]
            collector.query(
                f"c/{sha}/{owner}/{repo}",
                {
                    "version": "v0",
                    "type": "build_version",
                    "input": {
                        "version": "v0",
                        "source": {"type": "github", "owner": owner, "repo": repo},
                        "sha": sha,
                    },
                },
            )


def main() -> None:
    parser = ArgumentParser()
    parser.add_argument("-m", "--manifest-file", type=Path, default="manifest.json")
    parser.add_argument("-u", "--pump-url", default="http://127.0.0.1:5800")
    parser.add_argument("-b", "--batch-size", type=int, default=100)
    parser.add_argument("-i", "--poll-interval", type=int, default=2)
    parser.add_argument("-U", "--basic-username", default="foo")
    parser.add_argument("-P", "--basic-password", default="bar")
    parser.add_argument("--skip-version", action="store_true")
    parser.add_argument("--skip-build", action="store_true")
    args = parser.parse_args(namespace=Args())

    manifest = fetch_manifest(args.manifest_file)
    sources = get_sources_from_manifest(manifest)
    print(f"Found {len(sources)} sources in manifest")

    collector = Collector()
    derive_jobs(
        collector,
        sources,
        skip_version=args.skip_version,
        skip_build=args.skip_build,
    )

    with Progress(
        "{task.description}",
        BarColumn(),
        "[green]{task.completed:4d}/{task.total:4d}",
    ) as p:
        tp = p.add_task("pending", total=args.batch_size)
        tr = p.add_task("running", total=args.batch_size)
        tg = p.add_task("global", total=0)
        tv = p.add_task("version", total=0)
        tb = p.add_task("build", total=0)

        auth = (args.basic_username, args.basic_password)
        while len(collector.results) < len(collector.queries):
            pending, completed = collector.update(args.pump_url, auth, args.batch_size)
            derive_jobs(
                collector,
                sources,
                skip_version=args.skip_version,
                skip_build=args.skip_build,
            )

            n_pending = len(pending)
            n_running = sum(1 for v in pending.values() if v["started"])
            tg_queries, tg_results = collector.status_prefixed("a/")
            tv_queries, tv_results = collector.status_prefixed("b/")
            tb_queries, tb_results = collector.status_prefixed("c/")

            p.update(tp, completed=n_pending)
            p.update(tr, completed=n_running)
            p.update(tg, total=tg_queries, completed=tg_results)
            p.update(tv, total=tv_queries, completed=tv_results)
            p.update(tb, total=tb_queries, completed=tb_results)
            p.refresh()

            if not completed:
                time.sleep(args.poll_interval)


if __name__ == "__main__":
    main()
