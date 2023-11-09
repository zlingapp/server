-- add the guild_id column to the messages table from the channels table
ALTER TABLE messages 
    ADD guild_id text NOT NULL
    AS ((
        SELECT guild_id
        FROM channels
        WHERE channels.id = messages.channel_id
    ));
