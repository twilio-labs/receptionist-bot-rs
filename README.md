# receptionist-bot-rs

Slack bot for self-servicing automation of common or predictable tasks.

Receptionist bot aims to provide a no-code frontend for technical & non-technical users to easily automate away some of the tedious repetition that can permeate Slack as companies grow larger. Its scope is narrow: rather than replacing the first-party Slack Workflows system or becoming an entire ecosystem such as [SOCless](https://github.com/twilio-labs/socless), the Receptionist bot bridges the gap between the promise of Slack Workflows and the current state provided to users while also providing workflow automations for free-tier communities.

The app is designed to be highly extensible for new features in your workflows and is unopinionated with regards to its deployment infrastructure. 


- [Developing the bot in a local environment (with or without docker)](./docs/development.md#local-development)
- [How to deploy to AWS as a Server using Terraform](./docs/deployments.md#deployment-to-aws-as-a-standalone-server)
- [How to deploy to AWS as Serverless Functions using Terraform](./docs/deployments.md#deployment-to-aws-as-lambda-functions--api-gateway)
- [Setting up the Slack App](#creating-the-slack-apps-permissions-urls-slash-commands-etc)
- [Supported Features](#supported-features)
- [Known Bugs & Limitations](#known-bugs--limitations)
- [View UI Examples using Block Kit Builder](#View-UI-Examples-with-Block-Kit-Builder)
- [Special Thanks & Shoutouts](#special-thanks--shoutouts)


<img src=./manager.png width="450px" >


## Interacting with the Receptionist Bot
The app ships with a slash command `/rec-manage` that will display a UI for Creating, Editing, and Deleting Receptionist Workflow Responses

---

## Project Structure & Contributing

This project is a monorepo, meaning it contains multiple crates or (packages) in one repository. It consists of of the following crates:

- [`receptionist`](crates/receptionist) - The core library for making a receptionist bot implementation
- [`rec_server`](crates/rec_server/) - An implementation of Receptionist Bot as a standalone server or docker container
- `rec_lambda_<function>` - An implementation of Receptionist Bot as 3 serverless cloud functions on AWS lambda: [Slash Commands](./crates/rec_lambda_commands), [Interactions API](./crates/rec_lambda_interactions), and [Events API](./crates/rec_lambda_events)
- [`xtask`](crates/xtask/) - [xtask](https://github.com/matklad/cargo-xtask) exposes a small Rust CLI for build scripts related to the project, similar to a makefile or `npm run`. 
- [`terraform_aws`](./terraform_aws) - Terraform code for deploying either as a standalone server or serverless functions
- [`docs`](./docs) - A collection of guides on [deployments](docs/deployments.md), [project architecture](docs/architecture.md), and [development](docs/development.md)

---

# Supported Features

## Events that can trigger a response
| Event Origin             | Status        | Event Scope  | Origin Type
| ------------------------ | ------------- | ------------ | -----------
| Message sent to channel  | Done ✅    | Local (channel) | Slack Message
| Receptionist App mentioned in channel | Planned | Local (channel) | Slack Message
| Custom slash command: `/rec-cmd <my_command>`  | Planned | Global | Slash Command
| Webhook sent to Receptionist Server   | Not Planned | Global      | Server Event

## Conditions to check before triggering a response
| Condition        | Status        | Eligible Origin Types
| ---------------- | ------------- | ----------------------
| Matches a Regex  | Done ✅   | Slack Message
| Matches a Phrase | Done ✅   | Slack Message
| Is From a specific User | Planned | Slack Message

## Actions that a Response can take
| Action        | Status        | Eligible Origin Types
| ------------- | ------------- | ----------------------
React with Emoji (can trigger Slack Workflows) | Done ✅    | Slack Message
Send Message To Thread | Done ✅    | Slack Message
Tag Pagerduty oncall for <X> team in thread | Done ✅    | Slack Message
Send Message To Channel | Done ✅   | Slack Message
Forward message to a channel | Done ✅    | Slack Message
Tag Pagerduty oncall for <X> team in channel | Planned   | Slack Message
Send Message To User | Planned   | Slack Message
Tag User in thread | Planned | Slack Message
Tag User in Channel | Planned | Slack Message
Forward message to a user | Planned   | Slack Message
send webhook with custom payload | Planned   | Slack Message


## Known Bugs & Limitations

### Limitations
- App will not scan bot messages, hidden messages, or threaded messages to help prevent infinite loops or negatively impacting the signal-to-noise ratio of the channel.
  - listening for bot messages may be introduced in the future but only if the message condition is scoped to that single bot (user-specific message conditions have not been written yet, which blocks this bot message feature)


### Bugs
#### Listener Validation in Modal
**Intent:** If listener channel is empty in the Management Modal, we return an errors object response to Slack to display an error and prevent the view closing.

**Bug:** The view stays open correctly, However the error does not display.

**Situation Report:** We are confident the http response is being sent correctly to Slack because if anything is incorrect, Slack will display a connection error in the modal. We encountered this previously when the content-type was incorrect or when the `errors` object contained an invalid Block_ID. Since neither of those are happening now, I am inclined to believe this is either a bug or limitation on Slack's end with the Conversations Select Section Block
  - https://api.slack.com/surfaces/modals/using#displaying_errors

## View UI Examples with Block Kit Builder

1. Log into [Block Kit Builder](https://api.slack.com/tools/block-kit-builder)
2. Select **Modal Preview** from the dropdown in the top left
3. Copy a json template from [./crates/receptionist/tests/generated_blocks](./crates/receptionist/tests/generated_blocks) and paste it into the Block Kit json editor

## Special Thanks & Shoutouts
- [@abdolence](https://github.com/abdolence) for creating the excellent [`slack-morphism` library](https://github.com/abdolence/slack-morphism-rust).
  - `slack-morphism` strongly types nearly everything you need to work with Slack, including block kit models which are a huge pain point when doing highly reactive & dynamic UIs like the Receptionist Bot. This app would not have been possible without his work!
  - [@abdolence](https://github.com/abdolence) also [quickly responded to questions](https://github.com/abdolence/slack-morphism-rust/issues/69) with thorough answers and helpful tips
- [@davidpdrsn](https://github.com/davidpdrsn) for the [Axum](https://github.com/tokio-rs/axum) web application framework.
  - Axum is easy to use and is built with `hyper` & `tokio`, but also uses the `Tower` ecosystem so that you can share middleware/services/utilities, between any other framework that uses `hyper` or `tonic`. You can also often share code between server-side and client side implementations.
  - You can apply middlewares & Layers to only affect certain routes. In this App we use a Slack Verification Middleware from `slack-morphism` to protect our routes that expect to receive traffic from Slack, but not for other routes.
  - [@davidpdrsn](https://github.com/davidpdrsn) also [quickly helped out when I struggled to integrate an authentication middleware](https://github.com/tokio-rs/axum/discussions/625).
- [@yusuphisms](https://github.com/yusuphisms) for experimenting with the Pagerduty API to enable new bot features, helping incorporate Docker Compose & DynamoDB support, and setting up for integration tests.
- [@ubaniabalogun](https://github.com/ubaniabalogun) for thoroughly designing an effective DynamoDB model to enable a wide range of queries with a single Table.
- [@shadyproject](https://github.com/shadyproject) for helping load test and brainstorming deployment & database strategies.
- Everyone who has worked on [`strum`](https://github.com/Peternator7/strum) which powers up the already great Rust enums
- [@bryanburgers](https://github.com/bryanburgers) for his work on [https://github.com/zenlist/serde_dynamo](serde_dynamo) which makes it easier to use dynamoDB (and the alpha branch supports the brand new aws_rust_sdk!)

## Code of Conduct
https://github.com/twilio-labs/.github/blob/master/CODE_OF_CONDUCT.md 
