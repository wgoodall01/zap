-- Add migration script here

-- Track users.
create table user (
	id uuid primary key,
	created_at timestamptz not null default now(),
	updated_at timestamptz not null default now()
	
	name varchar not null,
	photo_url varchar,
);

-- Track Telegram user-logins.
create table login_tg (
	id uuid primary key,
	created_at timestamptz not null default now(),
	updated_at timestamptz not null default now(),
	
	-- Bind the user_id and tg_id together.
	user_id uuid not null references user(id) on delete cascade,
	tg_id bigint not null unique,
	
	username varchar not null,
	first_name varchar,
	last_name varchar,
	photo_url varchar,
);
