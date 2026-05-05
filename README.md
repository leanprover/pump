# Pump

A web server that runs [impeller](https://github.com/leanprover/impeller) jobs
on behalf of users. It exposes a declarative HTTP API for requesting impeller
results for Lean packages, manages an in-memory job queue, and persists results
as JSON to disk.
