# Web server

A tiny HTTP request router built on `std/http` and `std/template`: it matches a
request path to a handler and returns a response object.

## What it demonstrates

- Path routing with `match` + a `when` guard (`pathMatch("/users/:id", path)`).
- `std/http` response helpers: `json`, `text`, `badRequest`, `pathMatch`.
- HTML templating with `std/template`'s `render` (filling `${...}` holes).
- Named record types: a `Request` alias (`{ method, path, query, headers, body }`)
  threads the known input shape through every handler.

## Structure

- **`main.lin`** — builds a couple of requests and prints each routed response's status.
- **`router.lin`** — `router(req)`: dispatches a `Request` to the right handler by path.
- **`handlers.lin`** — `handleIndex` / `handleStatus` / `handleUser`: produce responses.
- **`views/index.lint`** — the HTML template rendered by `handleIndex`.
- **`router.test.lin` / `handlers.test.lin`** — assert the routed/handler status codes.
- **`template.test.lin`** — renders `index.lint` and asserts every `${...}` hole is filled.

## Typing note

`Request` is a precise named alias. Responses are kept as `Json`: they come
directly from `std/http`'s `json`/`text`/`badRequest` helpers (a dynamic header
map plus a serialized body), so there is no fixed response record to pin down.

## Run / Test

```bash
lin run examples/web-server/main.lin     # route a few requests, print statuses
lin test examples/web-server/            # router + handler + template suites
```
