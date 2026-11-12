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


def parse_version(version: str) -> Version | None:
    try:
        return Version.parse(version.removeprefix("v"))
    except ValueError:
        pass


def sorted_versions(versions: list[str]) -> list[str]:
    pv = [(p, v) for v in versions if (p := parse_version(v))]
    pv.sort()
    return [v for _, v in pv]


class Pump:
    def __init__(self, url: str, auth: tuple[str, str], poll_interval: int):
        self.url = url.rstrip("/")
        self.session = requests.Session()
        self.session.auth = auth
        self.poll_interval = poll_interval

    def request(self, method: str, path: str, **kwargs: Any) -> Any:
        res = self.session.request(method, f"{self.url}/v1/{path}", **kwargs)
        res.raise_for_status()
        return res.json()

    def run_batch(self, name: str, inputs: list[Any]) -> list[Any]:
        """Run all inputs in one batch and return the finished jobs' statuses."""
        print(f"Running {len(inputs)} {name} jobs")
        if not inputs:
            return []

        batch = self.request(
            "POST", "batch", json={"jobs": [{"input": i} for i in inputs]}
        )
        try:
            with Progress(
                "{task.description}",
                BarColumn(),
                "[green]{task.completed:4d}/{task.total:4d}",
            ) as p:
                tr = p.add_task("running", total=len(inputs))
                tf = p.add_task("finished", total=len(inputs))
                while True:
                    jobs = batch["jobs"].values()
                    p.update(tr, completed=sum(1 for j in jobs if j["started"]))
                    p.update(tf, completed=sum(1 for j in jobs if j["finished"]))
                    if not batch["running"]:
                        break
                    time.sleep(self.poll_interval)
                    batch = self.request("GET", f"batch/{batch['id']}")
        except KeyboardInterrupt:
            print("Canceling batch")
            self.request("DELETE", f"batch/{batch['id']}")
            raise

        return [j for j in batch["jobs"].values() if j["finished"]]


def main() -> None:
    parser = ArgumentParser()
    parser.add_argument("-m", "--manifest-file", type=Path, default="manifest.json")
    parser.add_argument("-u", "--pump-url", default="http://127.0.0.1:5800")
    parser.add_argument("-i", "--poll-interval", type=int, default=2)
    parser.add_argument("-U", "--basic-username", default="foo")
    parser.add_argument("-P", "--basic-password", default="bar")
    parser.add_argument("--skip-version", action="store_true")
    parser.add_argument("--skip-build", action="store_true")
    args = parser.parse_args(namespace=Args())

    manifest = fetch_manifest(args.manifest_file)
    sources = get_sources_from_manifest(manifest)
    print(f"Found {len(sources)} sources in manifest")

    pump = Pump(
        url=args.pump_url,
        auth=(args.basic_username, args.basic_password),
        poll_interval=args.poll_interval,
    )

    global_jobs = pump.run_batch(
        "global",
        [
            {
                "type": "analyze_global",
                "source": {"type": "github", "owner": owner, "repo": repo},
            }
            for owner, repo in sorted(set(sources))
        ],
    )
    if args.skip_version:
        return

    inputs = []
    for job in global_jobs:
        output = pump.request("GET", f"job/{job['id']}")["output"]
        if output is None:
            continue

        source = job["input"]["source"]
        tag_shas = output["git"]["tag_shas"]
        version_tags = (output.get("lake") or {}).get("version_tags")
        if version_tags is None:
            version_tags = output["git"]["version_tags"]

        for tag in version_tags:
            inputs.append(
                {"type": "analyze_version", "source": source, "sha": tag_shas[tag]}
            )

        if not args.skip_build and (sorted_tags := sorted_versions(version_tags)):
            inputs.append(
                {
                    "type": "build_version",
                    "source": source,
                    "sha": tag_shas[sorted_tags[-1]],
                }
            )

    # Multiple tags may point to the same commit, but pump rejects duplicates.
    inputs = list({json.dumps(i, sort_keys=True): i for i in inputs}.values())
    pump.run_batch("version and build", inputs)


if __name__ == "__main__":
    main()
