# Deployments

- [Deployment to AWS as a standalone server](#deployment-to-aws-as-a-standalone-server)
- [Deployment to AWS as Lambda Functions + API Gateway](#deployment-to-aws-as-lambda-functions--api-gateway)
- [To remove a Terraform deployment](#to-remove-a-terraform-deployment)


### Deployment to AWS as a standalone server
This repo contains a basic deployment setup striving for the cheapest possible server hosted application on AWS.

With Rust, the resource usage is so low that it can run at high throughput on the smallest & cheapest ec2 instance (t4g.nano) with plenty of headroom.

### Prerequisites
- awscli v2 installed
- terraform installed
- docker installed & running

#### Step 1 - Setup Secrets (already added to `.gitignore`)
1. Create a `./secrets` directory (this whole directory and sub files are already added to `.gitignore`)
2. Create a `./secrets/slack_secret` file that only contains your slack signing secret
3. Create a `./secrets/slack_token` file that only contains your slack bot token (starts with `xoxb-`)
4. Create a `./secrets/pagerduty_token` file that only contains your pagerduty api token (if you don't have one, you will need to comment out some lines in Terraform OR just put a fake token here)


#### Step 2 - Deploy Terraform Remote State Backend
1. `cd terraform_aws/remote-state`
2. `terraform init`
3. `terraform plan`, then `terraform apply` and approve

#### Step 3 - Deploy App with Terraform + Docker + AWScli
**Docker must be running and aws cli (v2) must be installed!**
1. `cd terraform_aws/server`
2. `terraform init`
3. `terraform plan`, then `terraform apply` and approve

#### Step 4 - Create or Update your Bot in Slack
- [Setting up the Slack App](#creating-the-slack-apps-permissions-urls-slash-commands-etc)
- [Interact with the Receptionist Bot](#interact-with-the-receptionist-bot)


### Deployment to AWS as Lambda Functions + API Gateway
If you prefer serverless, we can instead run the app as 3 lambda functions behind API Gateway

With Rust you get the fastest cold starts possible so there is no risk of Slack Trigger IDs timing out.

### Prerequisites
- terraform installed
- docker installed & running

#### Step 1 - Setup Secrets (already added to `.gitignore`)
1. Create a `./secrets` directory (this whole directory and sub files are already added to `.gitignore`)
2. Create a `./secrets/slack_secret` file that only contains your slack signing secret
3. Create a `./secrets/slack_token` file that only contains your slack bot token (starts with `xoxb-`)
4. Create a `./secrets/pagerduty_token` file that only contains your pagerduty api token (if you don't have one, you will need to comment out some lines in Terraform OR just put a fake token here)

#### Step 2 - Ensure lambda target is installed
1. `rustup target add aarch64-unknown-linux-musl`
2. (if on MacOS, you will need `musl-gcc`) `brew install filosottile/musl-cross/musl-cross`

#### Step 3 - Cross Compile Lambdas for Graviton2
1. Ensure docker is running `docker -v`
2. `cargo xtask build-lambda-all`
   1. This will build lambda packages in release mode inside a docker image, copy the binaries to the terraform directory, then rename the binaries to `bootstrap` which is required for AWS Lambdas using a custom runtime

#### Step 4 - Deploy Terraform Remote State Backend
1. `cd terraform_aws/remote-state`
2. `terraform init`
3. `terraform plan`, then `terraform apply` and approve

#### Step 5 - Deploy App with Terraform
1. `cd terraform_aws/serverless`
2. `terraform init`
3. `terraform plan`, then `terraform apply` and approve

#### Step 6 - Create or Update your Bot in Slack
- [Setting up the Slack App](#creating-the-slack-apps-permissions-urls-slash-commands-etc)
- [Interact with the Receptionist Bot](#interact-with-the-receptionist-bot)

## To remove a Terraform deployment
1. change directory to the terraform deployment you want to remove (remove the `remote-state` deployment last!)
   1. `cd terraform_aws/<serverless|server|remote-state>`
2. `terraform apply -destroy`


## Creating the Slack App & Permissions, URLs, Slash Commands, etc.
The Receptionist bot's Slack configuration is in a single `./manifest.yml` file can be pasted into your Slack App Manifest (either when creating a new app or modifying an existing one). You will just need to replace all instances of `<MY_BOT_URL_HERE>` in the `manifest.yml` with the actual URL of your deployed (or local) application.

## Interact with the Receptionist Bot
The app ships with a slash command `/rec-manage` that will display a UI for Creating, Editing, and Deleting Receptionist Workflow Responses