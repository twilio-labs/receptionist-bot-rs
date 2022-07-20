# https://paulmason.me/blog/2020-09-16-running-rust-in-lambda/
# https://paulmason.me/blog/2020-09-25-running-rust-in-lambda-2/
# https://github.com/emk/rust-musl-builder


terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 3.27"
    }
  }
  required_version = ">= 1.1.2"
}

provider "aws" {
  profile = "default"
  region  = "us-west-2"
}

### GLOBAL DATA 
### region = ${data.aws_region.current.name}
### account_id = ${data.aws_caller_identity.current.account_id}
data "aws_caller_identity" "current" {}
data "aws_region" "current" {}

terraform {
  backend "s3" {
    key            = "global/s3/terraform.tfstate"
    encrypt        = true
    dynamodb_table = "terraform-receptionist-locks"
  }
}


### SSM Secrets (from the ./secrets folder)
resource "aws_ssm_parameter" "slack_token_parameter" {
  name        = "/receptionist/slack/token"
  description = "slack bot token"
  type        = "SecureString"
  value       = file("${path.module}/../../secrets/slack_token")
}

resource "aws_ssm_parameter" "slack_secret_parameter" {
  name        = "/receptionist/slack/secret"
  description = "slack signing secret"
  type        = "SecureString"
  value       = file("${path.module}/../../secrets/slack_secret")
}

resource "aws_ssm_parameter" "pagerduty_token_parameter" {
  name        = "/receptionist/pagerduty/token"
  description = "pagerduty api token"
  type        = "SecureString"
  value       = file("${path.module}/../../secrets/pagerduty_token")
}

resource "aws_iam_policy" "password_policy_parameterstore" {
  name = "password-policy-parameterstore"

  policy = <<-EOF
  {
    "Version": "2012-10-17",
    "Statement": [
      {
        "Action": [
          "ssm:GetParameters"
        ],
        "Effect": "Allow",
        "Resource": [
          "${aws_ssm_parameter.slack_token_parameter.arn}",
          "${aws_ssm_parameter.slack_secret_parameter.arn}",
          "${aws_ssm_parameter.pagerduty_token_parameter.arn}"
        ]
      }
    ]
  }
  EOF
}

### Lambda
provider "archive" {}

data "archive_file" "events_lambda_zip" {
  source_file = "archives/rec_lambda_events/bootstrap"
  output_path = "archives/rec_lambda_events/events_lambda.zip"
  type        = "zip"
}

data "archive_file" "commands_lambda_zip" {
  source_file = "archives/rec_lambda_commands/bootstrap"
  output_path = "archives/rec_lambda_commands/commands_lambda.zip"
  type        = "zip"
}

data "archive_file" "interactions_lambda_zip" {
  source_file = "archives/rec_lambda_interactions/bootstrap"
  output_path = "archives/rec_lambda_interactions/interactions_lambda.zip"
  type        = "zip"
}

resource "aws_iam_role" "lambda_execution_role" {
  managed_policy_arns = [
    "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole",
    aws_iam_policy.password_policy_parameterstore.arn,
    aws_iam_policy.receptionist_table_policy.arn
  ]

  assume_role_policy = <<EOF
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Action": "sts:AssumeRole",
      "Principal": {
        "Service": "lambda.amazonaws.com"
      },
      "Effect": "Allow",
      "Sid": ""
    }
  ]
}
EOF
}


resource "aws_lambda_function" "events_api" {
  function_name = "receptionist_events_api"
  architectures = ["arm64"]

  source_code_hash = data.archive_file.events_lambda_zip.output_base64sha256
  filename         = data.archive_file.events_lambda_zip.output_path

  handler = "bootstrap"
  runtime = "provided.al2"

  role = aws_iam_role.lambda_execution_role.arn
  environment {
    variables = {
      SLACK_BOT_TOKEN      = "${aws_ssm_parameter.slack_token_parameter.value}"
      SLACK_SIGNING_SECRET = "${aws_ssm_parameter.slack_secret_parameter.value}"
      PAGERDUTY_TOKEN      = "${aws_ssm_parameter.pagerduty_token_parameter.value}"
    }
  }
}

resource "aws_lambda_function" "commands_api" {
  function_name = "receptionist_commands_api"
  architectures = ["arm64"]

  source_code_hash = data.archive_file.commands_lambda_zip.output_base64sha256
  filename         = data.archive_file.commands_lambda_zip.output_path

  handler = "bootstrap"
  runtime = "provided.al2"

  role = aws_iam_role.lambda_execution_role.arn
  environment {
    variables = {
      SLACK_BOT_TOKEN      = "${aws_ssm_parameter.slack_token_parameter.value}"
      SLACK_SIGNING_SECRET = "${aws_ssm_parameter.slack_secret_parameter.value}"
      PAGERDUTY_TOKEN      = "${aws_ssm_parameter.pagerduty_token_parameter.value}"
    }
  }
}

resource "aws_lambda_function" "interactions_api" {
  function_name = "receptionist_interactions_api"
  architectures = ["arm64"]

  source_code_hash = data.archive_file.interactions_lambda_zip.output_base64sha256
  filename         = data.archive_file.interactions_lambda_zip.output_path

  handler = "bootstrap"
  runtime = "provided.al2"

  role = aws_iam_role.lambda_execution_role.arn

  environment {
    variables = {
      SLACK_BOT_TOKEN      = "${aws_ssm_parameter.slack_token_parameter.value}"
      SLACK_SIGNING_SECRET = "${aws_ssm_parameter.slack_secret_parameter.value}"
      PAGERDUTY_TOKEN      = "${aws_ssm_parameter.pagerduty_token_parameter.value}"
    }
  }
}


resource "aws_apigatewayv2_api" "api" {
  name          = "receptionist_api"
  description   = "Receptionist Bot API"
  protocol_type = "HTTP"
}

resource "aws_apigatewayv2_integration" "events_api" {
  api_id           = aws_apigatewayv2_api.api.id
  integration_type = "AWS_PROXY"

  connection_type    = "INTERNET"
  description        = "Example.API"
  integration_method = "POST"
  integration_uri    = aws_lambda_function.events_api.invoke_arn

  payload_format_version = "2.0"
}

resource "aws_apigatewayv2_route" "events_api" {
  api_id = aws_apigatewayv2_api.api.id
  # route_key = "$default"
  route_key = "POST /slack/events"
  target    = "integrations/${aws_apigatewayv2_integration.events_api.id}"
}

resource "aws_apigatewayv2_integration" "commands_api" {
  api_id           = aws_apigatewayv2_api.api.id
  integration_type = "AWS_PROXY"

  connection_type    = "INTERNET"
  description        = "Example.API"
  integration_method = "POST"
  integration_uri    = aws_lambda_function.commands_api.invoke_arn

  payload_format_version = "2.0"
}

resource "aws_apigatewayv2_route" "commands_api" {
  api_id = aws_apigatewayv2_api.api.id
  # route_key = "$default"
  route_key = "POST /slack/commands"
  target    = "integrations/${aws_apigatewayv2_integration.commands_api.id}"
}

resource "aws_apigatewayv2_integration" "interactions_api" {
  api_id           = aws_apigatewayv2_api.api.id
  integration_type = "AWS_PROXY"

  connection_type    = "INTERNET"
  description        = "Example.API"
  integration_method = "POST"
  integration_uri    = aws_lambda_function.interactions_api.invoke_arn

  payload_format_version = "2.0"
}

resource "aws_apigatewayv2_route" "interactions_api" {
  api_id = aws_apigatewayv2_api.api.id
  # route_key = "$default"
  route_key = "POST /slack/interaction"
  target    = "integrations/${aws_apigatewayv2_integration.interactions_api.id}"
}

resource "aws_apigatewayv2_stage" "api" {
  api_id      = aws_apigatewayv2_api.api.id
  name        = "prod"
  auto_deploy = true
}

resource "aws_apigatewayv2_deployment" "api" {
  api_id      = aws_apigatewayv2_api.api.id
  description = "API deployment"

  triggers = {
    redeployment = sha1(join(",", tolist([
      jsonencode(aws_apigatewayv2_integration.events_api),
      jsonencode(aws_apigatewayv2_route.events_api),
      jsonencode(aws_apigatewayv2_integration.interactions_api),
      jsonencode(aws_apigatewayv2_route.interactions_api),
      jsonencode(aws_apigatewayv2_integration.commands_api),
      jsonencode(aws_apigatewayv2_route.commands_api),
      ]
    )))
  }

  lifecycle {
    create_before_destroy = true
  }
}

resource "aws_lambda_permission" "events_api" {
  statement_id  = "allow_apigw_invoke"
  function_name = aws_lambda_function.events_api.function_name
  action        = "lambda:InvokeFunction"
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_stage.api.execution_arn}/*"
}

resource "aws_lambda_permission" "commands_api" {
  statement_id  = "allow_apigw_invoke"
  function_name = aws_lambda_function.commands_api.function_name
  action        = "lambda:InvokeFunction"
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_stage.api.execution_arn}/*"
}
resource "aws_lambda_permission" "interactions_api" {
  statement_id  = "allow_apigw_invoke"
  function_name = aws_lambda_function.interactions_api.function_name
  action        = "lambda:InvokeFunction"
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_apigatewayv2_stage.api.execution_arn}/*"
}

output "invoke_url" {
  value = aws_apigatewayv2_stage.api.invoke_url
}


### DATABASE ############################################
resource "aws_dynamodb_table" "receptionist_bot_table" {
  name           = "receptionist_bot"
  billing_mode   = "PROVISIONED"
  read_capacity  = 2
  write_capacity = 2
  hash_key       = "pk"
  range_key      = "sk"

  attribute {
    name = "pk"
    type = "S"
  }

  attribute {
    name = "sk"
    type = "S"
  }

  global_secondary_index {
    name            = "InvertedIndex"
    hash_key        = "sk"
    range_key       = "pk"
    write_capacity  = 1
    read_capacity   = 1
    projection_type = "ALL"
  }

  tags = {
    Name    = "receptionist_bot_table"
    project = "receptionist_bot"
  }
}

resource "aws_iam_policy" "receptionist_table_policy" {
  name = "receptionist_table_policy"

  policy = <<-EOF
  {
    "Version": "2012-10-17",
    "Statement": [
      {
        "Action": [ "dynamodb:BatchGetItem",
                    "dynamodb:GetItem",
                    "dynamodb:GetRecords",
                    "dynamodb:Scan",
                    "dynamodb:Query",
                    "dynamodb:BatchWriteItem",
                    "dynamodb:PutItem",
                    "dynamodb:UpdateItem",
                    "dynamodb:DeleteItem",
                    "dynamodb:DescribeTable"
                    ],
        "Effect": "Allow",
        "Resource": [
          "${aws_dynamodb_table.receptionist_bot_table.arn}",
          "${aws_dynamodb_table.receptionist_bot_table.arn}/*"
        ]
      }
    ]
  }
  EOF
}
