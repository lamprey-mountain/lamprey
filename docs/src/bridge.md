# discord bridge

the discord bridge automatically forwards messages between discord and lamprey

## how to setup your own instance

Create a new config file; look at
[the example config](./bridge-discord-config.toml).

Create a discord bot:

1. create a
   [new discord application](https://canary.discord.com/developers/applications)
2. in settings > install > installation contexts, disable user installation
3. in settings > bot, click "reset token" and copy the token to `discord_token`
   in config.toml
4. also consider unchecking "public bot" here if you're only using the bridge
   for yourself
5. enable all privileged intents (presence, server members, message content)

Create a lamprey bot:

1. go to user settings > applications and create an application
2. check "bridge"
3. create a session and copy it to `lamprey_token`

Currently, the only supported way to run the bot is with docker. See the
[example docker compose config](./example-docker-compose.yaml).

## how to use

1. install the bot on discord with the url
   `https://discord.com/oauth2/authorize?client_id=YOUR_ID_HERE&scope=bot&permissions=2252196829981776`
2. install the bot on lamprey (TODO: in settings > applications)
3. run `/link guild ROOM_UUID`
4. TODO: confirm link from lamprey

- set `continuous` to true to proactively create new threads/channels and link
  them
- set `backfill` to send the full message history from all discord channels into
  the newly created lamprey threads
- you can use `link channel` to link a single channel
