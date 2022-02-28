# Architecture

- [Architecture](#architecture)
  - [Background](#background)
    - [Receptionist Bot Features vs Other Slack Automation Apps](#receptionist-bot-features-vs-other-slack-automation-apps)
  - [Design Philosophy](#design-philosophy)
  - [Code Map](#code-map)
  - [Components](#components)
  - [High-Level Features](#high-level-features)

## Background 

The Receptionist Bot is an experimental app designed as a lightweight, scalable Slack workflow automation system after I discovered multiple attempts to solve this problem across various companies with only limited degrees of success. Rather than replacing the first-party Slack Workflows system or becoming an entire wholesale automation platform such as [SOCless](https://github.com/twilio-labs/socless), the Receptionist bot can tap into both (or neither) to provide a full-featured yet easy user interface for Slack automations.

### Receptionist Bot Features vs Other Slack Automation Apps
- No-Code interface, easier to use than Slack Workflows
  - Even though many users may find it easy, we have seen non-technical users struggle to stop and learn Workflows well enough to implement and maintain a useful automation playbook
- Enhances Slack Workflows
  - Many users can write Slack Workflows but quickly run against some core limitations such as its inability to be automatically triggered by messages in a channel
  - Receptionist Bot can parse messages for you and trigger any existing Slack Workflow, or you can choose to just use Receptionist Actions instead.
- $ Extremely Cheap, plus you own your data $
  - Receptionist Bot can easily run on the cheapest AWS ec2 (`t4g.nano`) or on free-tier serverless functions, or self-hosted on your own servers.
  - Many companies exist to solve this Slack automation problem, but they charge per-user pricing and limit the amount of automations that can run per month. This gets expensive quickly as your company scales.
  - Slack data can be extremely sensitive. By running your own app theres no worry of data breaches against 3rd party companies impacting your own business

## Design Philosophy
- **Open, Unopinionated Infrastructure**
  - Don't lock the app into AWS or cloud technologies
    - We have thorough examples for deploying as a standalone server with Docker on AWS, as serverless functions on AWS, and local development (with & without Docker).
    - Examples for [GCP](https://cloud.google.com), [fly.io](https://fly.io) & other environments would be welcome.
  - Don't lock the app to a specific database type
    - Database is selected via `cargo` feature flags.
    - If a user would rather use postgres instead of dynamoDB, they can create a postgres option in the `./receptionist/src/database` directory, which only requires writing the ~ 5 CRUD operation functions needed for that new database
- **Write code in a manner that maximizes the benefits of Rust to check errors with the compiler instead of elaborate unit testing**
  - Because the Bot aims to provide a first-class dynamic UI inside Slack, we must be careful to ensure that we are programming in a manner that allows the compiler to check our work _before_ we get a runtime error when attempting to deserialize the modal submission.
  - Enums are your friend. The Slack modals API requires many constant strings for routing inputs to the correct place, and we need to handle every edge case. By utilizing enums whenever possible we can allow Rust to check our work instead of tedious manual testing.
- **Write code for future extensibility even if its not needed yet**
  - **Example**: The `ReceptionistResponse` struct is the complete description of a single automation workflow, and is made up of Listeners, Conditions, and Actions. We know that users will want more types for each of these, so they are all enums.
  - **Example**: Even though the app can currently only support one action and one condition per workflow, the data model stores them stored in vectors and tracks their indices during updates to ensure that we do not prevent allowing multiple actions per workflow in the future.
  - Use `strum` crate to supercharge enums and reduce repeat code. 
  


## Code Map
- [`./receptionist`](./receptionist) - Rust library for all common code powering the Receptionist Bot.
  - [`receptionist/config`](./receptionist/config) - The bot's configuration struct and initialization code.
  - [`receptionist/database`](./receptionist/src/database/mod.rs) - code for all database types. database selection is controlled by cargo feature flags, with all databases using the same function names
  - [`receptionist/manager`](./receptionist/src/manager/mod.rs) - code for the Slack modal where users can manage their automations
  - [`receptionist/pagerduty`](./receptionist/src/pagerduty/mod.rs) - Pagerduty client to find the oncall user for a specific team
  - [`receptionist/response`](./receptionist/src/response/mod.rs) - Core code for the Receptionist's main data model, the Response. Automations are basically all boiled down to a single Response struct.
  - [`receptionist/slack`](./receptionist/src/slack/mod.rs) - All Slack code that isnt specific to the design of the Management interface
- [`./rec_server`](./rec_server) - Rust (Axum) Webserver for deploying the Receptionist Bot as a standalone server application.
- `./rec_lambda_commands`, `./rec_lambda_events`, `./rec_lambda_interactions`
  - Rust binaries for deploying the Receptionist Bot as serverless Lambda Functions behind an AWS API Gateway.
  - Each function covers a specific http route: `/commands`, `/events`, `/interactions`
- [`./terraform_aws`](./terraform_aws) - contains 3 different terraform deployments for the bot
  1. `terraform_aws/remote-state` is required to deploy the bot in either server or serverless mode
  2. `terraform_aws/server` will deploy the bot using ECS on a `t4g.nano` EC2 instance
  3. `terraform_aws/serverless` will deploy the bot as 3 Lambda Functions and an API Gateway
- [`./xtask`](./xtask) - [common use cases typically reserved for makefiles](https://github.com/matklad/cargo-xtask/)
  - **To view available commands, run**:
    ```sh
      cargo xtask help
    ```



## Components
- DynamoDB is used for persistent app data such as Authentication, Channel Configurations for user-configured actions & conditions, 


## High-Level Features
- Process the following Slack Event Subscriptions:
  - [ ] `message` : search messages for user-defined Match values and do their configured action
- Users can create a Condition for the bot to search messages for.
- Users can create an Action for the bot to take when any of their Condition criteria is met.
- Supported Match features:
  - [ ] Look for a phrase within a message
  - [ ] Parse message using a Regex string
- Supported  features:
  - [ ] React to a message with an emoji (**useful for _triggering_ Slack Workflows**)
  - [ ] Find a specific PagerDuty team & tag their oncall user in the thread
  - [ ] Find a specific PagerDuty team & page them
  - [ ] Ping a specific user with a threaded message (duplicate of Slack workflows)
  - [ ] Reply with a predefined message like an FAQ (duplicate of Slack workflows)
-  slash command keywords are user configurable via env, uses default if no env set
  - [ ] /config, /update, etc
- slash commands
  - [ ] /rec-config : CRUD modal for Matches & Actions
  - [ ] /rec-list   : ephemeral message of all configured Matches & Actions

