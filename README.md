# Cartobase

Remote chunk-sync server and web dashboard for
[VoxelMap-x-SeedMapper](https://github.com/cev-api/VoxelMap-x-SeedMapper).

Cartobase is the persistent, self-hosted backend behind the mod's `chunksync`
feature. Instead of exporting a snapshot to a throwaway file host, clients push
their explored / new / old chunk sets (and waypoints) as small deltas to a
Cartobase server and pull everyone else's back live. It's a single Rust binary
plus Postgres, shipped as one Docker image, with a web panel for auth tokens and
stats.


## Storage model

Chunk sets are not stored one row per chunk. Each `(crew, world, dimension,
category, player)` stream is split into **containers** of `32 × 32` chunks; a
container is a 128-byte bitmap (1024 bits). Setting a chunk ORs a bit. This keeps
hundreds of millions of chunks compact and makes range queries and deltas
container-granular, mirroring the client's own `ExploredDiskStore`.

Every container write and every waypoint write stamps a row `seq` drawn from a
global sequence, so clients pull "everything since cursor N" cheaply.

The Cartobase transport sends coordinates **in the clear over TLS**, authorized
by a per-player bearer token (Will add encryption in the future)

## Data model

- **crew** — a shared namespace.
- **token** — a bearer token bound to one crew and one player name, role
  `member` or `admin`. Members sync; admins also manage tokens and view the
  dashboard.
- **chunk_containers** — the bitmap store described above.
- **waypoints** — per-crew, per-world waypoints.

## HTTP API

Sync (bearer token required):

| Method | Path | Purpose |
| ------ | ---- | ------- |
| `POST` | `/api/v1/sync/chunks` | push chunk-coordinate deltas per category |
| `GET`  | `/api/v1/sync/chunks?world=&dim=&since=` | pull changed containers since a cursor |
| `POST` | `/api/v1/sync/waypoints` | push waypoint upserts / deletes |
| `GET`  | `/api/v1/sync/waypoints?world=&since=` | pull changed waypoints since a cursor |
| `GET`  | `/api/v1/health` | liveness |

Admin (admin token required):

| Method | Path | Purpose |
| ------ | ---- | ------- |
| `GET`  | `/api/v1/admin/stats` | crew/chunk/player/waypoint counts |
| `GET`  | `/api/v1/admin/crews` | list crews |
| `POST` | `/api/v1/admin/crews` | create a crew |
| `GET`  | `/api/v1/admin/tokens` | list tokens (hashes only) |
| `POST` | `/api/v1/admin/tokens` | mint a token (returned once, in the clear) |
| `POST` | `/api/v1/admin/tokens/{id}/revoke` | revoke a token |

The dashboard (`/`) is a static page that calls the admin API with an admin
token you paste in.

## Running

```sh
cp .env.example .env
docker compose up --build
```

On first run, if no admin token exists, Cartobase creates the `default` crew and
prints a fresh admin token to the logs (or uses `CARTOBASE_ADMIN_TOKEN`). Open
`http://localhost:8080/`, paste the token, and mint member tokens for the crew.

## Development

```sh
cp .env.example .env         # point DATABASE_URL at a local postgres
cargo run
```

Migrations in `migrations/` run automatically on startup.
