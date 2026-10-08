create extension if not exists citext;

create table users (
    id uuid primary key default uuidv7(),
    handle citext not null unique check (handle::text ~ '^[a-z0-9_.]{3,30}$'),
    display_name text not null,
    role text not null default 'user' check (role in ('admin', 'user')),
    timezone text not null default 'UTC',
    created_at timestamptz not null default now(),
    last_seen_at timestamptz not null default now()
);

create table sessions (
    id uuid primary key default uuidv7(),
    user_id uuid not null references users (id) on delete cascade,
    token_hash bytea not null unique,
    kind text not null check (kind in ('browser', 'device')),
    name text,
    created_at timestamptz not null default now(),
    last_used_at timestamptz not null default now(),
    expires_at timestamptz
);

create index sessions_user_id_idx on sessions (user_id);

create table entries (
    id uuid primary key default uuidv7(),
    user_id uuid not null references users (id) on delete cascade,
    created_at timestamptz not null default now()
);

create index entries_user_id_created_at_idx on entries (user_id, created_at desc);
