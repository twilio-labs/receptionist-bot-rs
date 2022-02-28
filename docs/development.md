
## Local Development
### Prerequisites
- ngrok
- docker (optional, used for simple spinup of local development with dynamodb)

There are two modes of local development: Docker-Compose w/DynamoDB vs. Cargo (temporary DB us)

#### Step 1 - Setup `.env` File
```
  SLACK_BOT_TOKEN=<xoxb-1234567>
  SLACK_SIGNING_SECRET=<slack-signing-secret>
  PAGERDUTY_TOKEN=<api_token> (Optional)
```

#### Step 2 - Start the bot (either with docker or cargo)
- To use dynamoDB & docker, run `docker compose up --build`
- To use a hashmap as a temporary database and test without docker `cargo run --bin receptionist_server --features="tempdb, ansi" --no-default-features`


#### Step 3 - Start ngrok and connect Slack to it 
1. In a new Terminal, at the ngrok installation directory: `ngrok http 3000` or `./ngrok http --region=us --hostname=<custom_name>.ngrok.io 3000`
2. Get the https url from ngrok and replace all instances of `<MY_BOT_URL_HERE>` in the `./manifest.yml`
3. Paste the updated `manifest` in your Slack App @ https://api.slack.com/apps - ([Setting up the Slack App](#creating-the-slack-apps-permissions-urls-slash-commands-etc))
4. It may ask you to verify the Event Subscription URL, if your local bot is running this check should pass.


## Creating the Slack App & Permissions, URLs, Slash Commands, etc.
The Receptionist bot's Slack configuration is in a single `./manifest.yml` file can be pasted into your Slack App Manifest (either when creating a new app or modifying an existing one). You will just need to replace all instances of `<MY_BOT_URL_HERE>` in the `manifest.yml` with the actual URL of your deployed (or local) application.
