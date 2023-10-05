BEGIN;
CREATE TABLE users (
    id          text        NOT NULL PRIMARY KEY,
    name        text        NOT NULL,
    email       text,
    password    text,
    avatar      text        NOT NULL,
    friends     text[]      NOT NULL DEFAULT array[]::text[],
    created_at  timestamp   NOT NULL DEFAULT now(),
    updated_at  timestamp   NOT NULL DEFAULT now(),
    bot         boolean     NOT NULL DEFAULT false
);
CREATE TABLE guilds (
    id          text        NOT NULL PRIMARY KEY,
    name        text        NOT NULL,
    owner       text        NOT NULL,
    created_at  timestamp   NOT NULL DEFAULT now(),
    updated_at  timestamp   NOT NULL DEFAULT now(),
    permissions json        NOT NULL DEFAULT '{}'::json,
    icon        text
);
CREATE TABLE members (
    user_id     text        NOT NULL,
    guild_id    text        NOT NULL,
    joined_at   timestamp   NOT NULL DEFAULT now(),
    permissions json        NOT NULL DEFAULT '{}'::json,
    roles       text[]      NOT NULL DEFAULT array[]::text[],
    nickname    text,
    PRIMARY KEY (user_id, guild_id)
);
CREATE TYPE channel_type AS ENUM ('text', 'voice');
CREATE TABLE channels (
    id          text            NOT NULL PRIMARY KEY,
    type        channel_type    NOT NULL,
    name        text            NOT NULL,
    guild_id    text            NOT NULL,
    created_at  timestamp       NOT NULL DEFAULT now(),
    updated_at  timestamp       NOT NULL DEFAULT now(),
    permissions json            NOT NULL DEFAULT '{}'::json
);
CREATE TABLE messages (
    id          text        NOT NULL PRIMARY KEY,
    guild_id    text        NOT NULL, -- this might be redundant
    channel_id  text        NOT NULL,
    user_id     text        NOT NULL,
    content     text,
    created_at  timestamp   NOT NULL DEFAULT now(),
    updated_at  timestamp   NOT NULL DEFAULT now(),
    attachments json
);
CREATE TABLE roles (
    id          text        NOT NULL PRIMARY KEY,
    name        text        NOT NULL,
    guild_id    text        NOT NULL,
    created_at  timestamp   NOT NULL DEFAULT now(),
    updated_at  timestamp   NOT NULL DEFAULT now(),
    created_by  text        NOT NULL,
    permissions json        NOT NULL DEFAULT '{}'::json
);
CREATE TABLE tokens (
    token_id    text        NOT NULL,
    nonce       text        NOT NULL,
    user_id     text        NOT NULL,
    expires_at  timestamp   NOT NULL,
    user_agent  text        NOT NULL DEFAULT 'Unknown',
    PRIMARY KEY (nonce, user_id)
);
CREATE TABLE bots (
    id          text        NOT NULL,
    owner_id    text        NOT NULL,
    created_at  timestamp   NOT NULL DEFAULT now(),
    updated_at  timestamp   NOT NULL DEFAULT now(),
    PRIMARY KEY (owner_id, id)
);
COMMIT;