CREATE TABLE friend_requests (
    from_user   text        NOT NULL,
    to_user     text        NOT NULL,
    PRIMARY KEY (from_user, to_user)
);