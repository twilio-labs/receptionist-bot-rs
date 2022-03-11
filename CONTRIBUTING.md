# Contributing to twilio-labs/receptionist-bot-rs

## Project Structure

This project is a monorepo, meaning it contains multiple crates or (packages) in one repository. It consists of of the following crates:

- [`receptionist`](crates/receptionist) - The core library for making a receptionist bot implementation
- [`rec_server`](crates/rec_server/) - An implementation of Receptionist Bot as a standalone server or docker container
- `rec_lambda_<function>` - An implementation of Receptionist Bot as 3 serverless cloud functions on AWS lambda: [Slash Commands](./crates/rec_lambda_commands), [Interactions API](./crates/rec_lambda_interactions), and [Events API](./crates/rec_lambda_events)
- [`xtask`](crates/xtask/) - [xtask](https://github.com/matklad/cargo-xtask) exposes a small Rust CLI for build scripts related to the project, similar to a makefile or `npm run`. 
- [`terraform_aws`](./terraform_aws) - Terraform code for deploying either as a standalone server or serverless functions
- [`docs`](./docs) - A collection of guides on [deployments](docs/deployments.md), [project architecture](docs/architecture.md), and [development](docs/development.md)

---

## Code of Conduct

Please be aware that this project has a [Code of Conduct](https://github.com/twilio-labs/.github/blob/master/CODE_OF_CONDUCT.md). The tldr; is to just be excellent to each other ❤️

## Licensing

All third party contributors acknowledge that any contributions they provide will be made under the same open source license that the open source project is provided under.
