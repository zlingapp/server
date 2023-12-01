CREATE TABLE dmchannels (
    id          text            NOT NULL PRIMARY KEY,
    from_user   text            NOT NULL,
    to_user     text            NOT NULL,
    created_at  timestamp       NOT NULL DEFAULT now(),
    updated_at  timestamp       NOT NULL DEFAULT now()
);
