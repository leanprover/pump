# Pump

A web server that runs [impeller](https://github.com/leanprover/impeller) jobs
on behalf of users.

## API

All endpoints accept and return JSON. The data types are documented in the code.
Unknown fields are rejected. Authenticated endpoints use HTTP basic auth.

| Method   | Endpoint         | Auth | Description                                                 | Request                                                 | Response                              |
| -------- | ---------------- | ---- | ----------------------------------------------------------- | ------------------------------------------------------- | ------------------------------------- |
| `POST`   | `/v1/batch`      | yes  | Create a new batch of jobs, optionally with a callback URL. | [`PostRequest`](src/server/api/batch.rs)                | [`Batch`](src/server/data.rs)         |
| `GET`    | `/v1/batch/{id}` | yes  | Get a batch's current status.                               | -                                                       | [`Batch`](src/server/data.rs)         |
| `DELETE` | `/v1/batch/{id}` | yes  | Abort a batch.                                              | -                                                       | [`Batch`](src/server/data.rs)         |
| `GET`    | `/v1/job/{id}`   | no   | Retrieve the full results of a single job.                  | -                                                       | [`JobResult`](src/server/data.rs)     |
| `GET`    | `/v1/queue`      | no   | Get the list of jobs currently in the queue.                | -                                                       | [`GetReply`](src/server/api/queue.rs) |
| `GET`    | `/v1/queue/{id}` | no   | Get the current status of a job in the queue.               | [`GetRunQuery`](src/server/api/queue.rs) (query string) | [`Run`](src/server/data.rs)           |
