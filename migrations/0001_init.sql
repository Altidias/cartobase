create table crews (
    id uuid primary key,
    name text not null unique,
    created_at timestamptz not null default now()
);

create table tokens (
    id uuid primary key,
    crew_id uuid not null references crews(id) on delete cascade,
    token_hash text not null unique,
    player_name text not null,
    role text not null default 'member',
    created_at timestamptz not null default now(),
    last_seen_at timestamptz,
    revoked boolean not null default false
);

create sequence chunk_change_seq;
create sequence waypoint_change_seq;

-- one row per container; bitmap holds container_chunks^2 bits of a single category
create table chunk_containers (
    crew_id uuid not null references crews(id) on delete cascade,
    world_key text not null,
    dimension text not null,
    category smallint not null,
    player_slug text not null,
    container_x integer not null,
    container_z integer not null,
    bitmap bytea not null,
    chunk_count integer not null default 0,
    seq bigint not null,
    updated_at timestamptz not null default now(),
    primary key (crew_id, world_key, dimension, category, player_slug, container_x, container_z)
);

create index chunk_containers_pull_idx
    on chunk_containers (crew_id, world_key, dimension, seq);

create table waypoints (
    crew_id uuid not null references crews(id) on delete cascade,
    world_key text not null,
    wp_id text not null,
    name text not null,
    x integer not null,
    y integer not null,
    z integer not null,
    dimensions text not null default '',
    color integer not null default 0,
    icon text not null default '',
    beacon boolean not null default false,
    deleted boolean not null default false,
    author text not null default '',
    seq bigint not null,
    updated_at timestamptz not null default now(),
    primary key (crew_id, world_key, wp_id)
);

create index waypoints_pull_idx
    on waypoints (crew_id, world_key, seq);
