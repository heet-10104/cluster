-- Add migration script here
create table api_info(
    id serial primary key, 
    api text unique not null,
    hits integer not null default 0,
    error_rate real default 0.0,
    errors integer[]
)