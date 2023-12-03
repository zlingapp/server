CREATE TABLE invites (
    code        text        NOT NULL PRIMARY KEY,
    guild_id    text        NOT NULL REFERENCES guilds (id) ON DELETE cascade,
    inviter     text        NOT NULL REFERENCES users (id) ON DELETE set null,
    uses        integer     DEFAULT null, -- This is nullable for infinite invites
    expires_at  timestamp   DEFAULT null -- nullable for no expiration date
);