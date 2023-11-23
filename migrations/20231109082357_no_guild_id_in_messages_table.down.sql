-- add the guild_id column to the messages table
ALTER TABLE messages ADD COLUMN guild_id text;

-- set the guild_id column to the guild_id from the channels table
UPDATE messages SET guild_id=(SELECT guild_id FROM channels WHERE channels.id = channel_id);

-- set the guild_id column to not null
ALTER TABLE messages ALTER COLUMN guild_id SET NOT NULL;