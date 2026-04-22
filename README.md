# Pump

A web server that runs [impeller](https://github.com/leanprover/impeller) jobs
on behalf of users. It exposes a declarative HTTP API for requesting analyze and
build results for Lean packages, manages an in-memory job queue, and persists
results as JSON to disk.

## Overview

Impeller provides two CLI commands:

- **`impeller-analyze-global`**: Collect global metadata for a lake package.
- **`impeller-analyze-version`**: Collect metadata for a specific version of a
  lake package.

Pump wraps these as a service: Users declare which results they want, pump
returns what it already has and queues the rest.

## API

### `POST /query`

Query pump declaratively: The user supplies a list of **input configurations**
specifying things like the command, package, version/tag, or whether to try
building the package.

The server responds with two lists:

- **`completed`**: Completed results where the server already has the data.
- **`pending`**: Status reports for inputs that are queued or in progress.

The server ensures that all configurations will eventually be completed by
adding them to the queue if necessary.

### `GET /queue`

Returns the current state of the job queue, including in-progress jobs, as a
list of status reports in the same format as the `pending` list from `/query`.

### Types

```ts
// Only one type of source supported for now
type Source = {
  type: "github",
  owner: str,
  name: str,
};

type AnalyzeGlobal = {
  type: "analyze-global",
  source: Source,
};

type AnalyzeVersion = {
  type = "analyze-version",
  source: Source,
  build?: bool, // Whether to attempt to build the repo
  test?: bool, // Whether to attempt to test the repo
  lint?: bool, // Whether to attempt to lint the repo
};

type InputConfiguration = AnalyzeGlobal | AnalyzeVersion;

type Job = {
  input: InputConfiguration,
  running_since?: Timestamp
};

type Result = {
  input: InputConfiguration,
  output: Any, // Output of impeller
};

type QueryRequest = {
  jobs: InputConfiguration[],
};

type QueryReply = {
  completed: Result[],
  pending: Job[],
};

type QueueReply = {
  pending: Job[],
};
```

## Results cache

Results are persisted to disk at `<data-dir>/<owner>/<repo>/<hash>.json` where
`<hash>` is a deterministic hash of the job's input configuration. Each result
file has the same structure as the `Result` type defined above.

`result.input` must result in the same hash as the filename when hashed. This
makes it possible to prune result files based on arbitrary criteria in the
future. Results are kept indefinitely for now.

## Queue and parallelism

The job queue lives **in memory only**. If the server restarts, pending and
in-progress jobs are lost. Cached results on disk are unaffected.

Pump is configured with a **thread limit**. Each job type costs a certain number
of threads:

- **Analyze** jobs cost 1 thread\*.
- **Build** jobs cost more (default configurable in config file).
- There are exceptions for some large repos like mathlib that may take a large
  chunk of available threads.

This allows many analyze jobs to run in parallel, or a smaller number of build
jobs, with the scheduler packing jobs up to the thread limit.
