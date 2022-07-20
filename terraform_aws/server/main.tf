# https://klotzandrew.com/blog/deploy-an-ec2-to-run-docker-with-terraform
# https://www.oneworldcoders.com/blog/using-terraform-to-provision-amazons-ecr-and-ecs-to-manage-containers-docker
# https://www.sufle.io/blog/keeping-secrets-as-secret-on-amazon-ecs-using-terraform
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
  region = "us-west-2"
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


###------------------------------------------------
### Secrets (from the ./secrets folder)
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


### ECR #######################
resource "aws_ecr_repository" "receptionist_bot_container" {
  name                 = "receptionist_bot_container"
  image_tag_mutability = "MUTABLE"

  tags = {
    project = "receptionist_bot"
  }
}

resource "aws_ecr_repository_policy" "receptionist-repo-policy" {
  repository = aws_ecr_repository.receptionist_bot_container.name
  policy     = <<EOF
  {
    "Version": "2008-10-17",
    "Statement": [
      {
        "Sid": "adds full ecr access to the demo repository",
        "Effect": "Allow",
        "Principal": "*",
        "Action": [
          "ecr:BatchCheckLayerAvailability",
          "ecr:BatchGetImage",
          "ecr:CompleteLayerUpload",
          "ecr:GetDownloadUrlForLayer",
          "ecr:GetLifecyclePolicy",
          "ecr:InitiateLayerUpload",
          "ecr:PutImage",
          "ecr:UploadLayerPart"
        ]
      }
    ]
  }
  EOF
}

resource "null_resource" "build_image" {
  provisioner "local-exec" {
    command = "aws -h && docker build -f '../../dockerfiles/Dockerfile.aws' ../../ -t '${aws_ecr_repository.receptionist_bot_container.repository_url}:aws'"
  }

  depends_on = [
    aws_ecr_repository.receptionist_bot_container,
  ]
}

resource "null_resource" "push_image_to_ecr" {
  provisioner "local-exec" {
    command = "aws ecr get-login-password --region ${data.aws_region.current.name} | docker login --username AWS --password-stdin '${data.aws_caller_identity.current.account_id}.dkr.ecr.${data.aws_region.current.name}.amazonaws.com' && docker push '${aws_ecr_repository.receptionist_bot_container.repository_url}:aws'"
  }

  depends_on = [
    aws_ecr_repository.receptionist_bot_container,
    null_resource.build_image,
  ]
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


#### ECS ##################################
resource "aws_ecs_cluster" "receptionist_cluster" {
  name = "receptionist_cluster"
}

resource "aws_ecs_task_definition" "receptionist_task_definition" {
  family             = "receptionist_family"
  execution_role_arn = aws_iam_role.ecs_agent.arn
  ## might fix no creds issue in container?
  task_role_arn = aws_iam_role.ecs_agent.arn
  # network_mode             = "awsvpc"
  requires_compatibilities = ["EC2"]
  # memory                   = "400"
  # cpu                      = "2048"

  container_definitions = <<EOF
[
  {
    "name": "receptionist-container",
    "image": "${aws_ecr_repository.receptionist_bot_container.repository_url}:aws",
    "essential": true,
    "memoryReservation" : 150,
    "secrets": [
      {
        "name": "SLACK_BOT_TOKEN",
        "valueFrom": "${aws_ssm_parameter.slack_token_parameter.arn}"
      },
      {
        "name": "SLACK_SIGNING_SECRET",
        "valueFrom": "${aws_ssm_parameter.slack_secret_parameter.arn}"
      },
      {
        "name": "PAGERDUTY_TOKEN",
        "valueFrom": "${aws_ssm_parameter.pagerduty_token_parameter.arn}"
      }
    ],
    "environment" : [
      {"name" : "AWS_REGION" , "value" : "${data.aws_region.current.name}"}
    ],
    "logConfiguration": {
      "logDriver": "awslogs",
      "options": {
          "awslogs-group": "${aws_cloudwatch_log_group.receptionist_logs_group.name}",
          "awslogs-region": "${data.aws_region.current.name}",
          "awslogs-stream-prefix": "receptionist-stream"
      }
    },
    "portMappings": [
      {
        "containerPort": 3000,
        "hostPort": 80
      },
      {
        "containerPort": 3000,
        "hostPort": 3000
      },
      {
        "containerPort": 3000,
        "hostPort": 443
      }
    ]
  }
]
EOF
}

resource "aws_ecs_service" "receptionist-service" {
  name            = "receptionist-app"
  cluster         = aws_ecs_cluster.receptionist_cluster.id
  task_definition = aws_ecs_task_definition.receptionist_task_definition.arn
  launch_type     = "EC2"
  desired_count   = 1
  # network_configuration {
  #   subnets          = ["subnet-05t93f90b22ba76qx"]
  #   assign_public_ip = true
  # }
}

resource "aws_launch_configuration" "ecs_launch_config" {
  image_id             = "ami-0c972b8adbd52bea0"
  iam_instance_profile = aws_iam_instance_profile.ecs_agent.arn
  # security_groups             = [aws_security_group.ecs_task.id]
  security_groups             = [aws_security_group.ecs_sg.id]
  instance_type               = "t4g.nano"
  associate_public_ip_address = true
  user_data                   = "#!/bin/bash\necho 'ECS_CLUSTER=receptionist_cluster \n ECS_ENABLE_TASK_IAM_ROLE=true \n'>> /etc/ecs/ecs.config"
}


resource "aws_autoscaling_group" "failure_analysis_ecs_asg" {
  name                 = "asg"
  vpc_zone_identifier  = [aws_subnet.pub_subnet.id]
  launch_configuration = aws_launch_configuration.ecs_launch_config.name

  desired_capacity          = 1
  min_size                  = 1
  max_size                  = 2
  health_check_grace_period = 300
  health_check_type         = "EC2"
}


data "aws_iam_policy_document" "ecs_agent" {
  statement {
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["ec2.amazonaws.com", "ecs.amazonaws.com", "ecs-tasks.amazonaws.com"]
    }
  }
}

resource "aws_iam_role" "ecs_agent" {
  name               = "ecs-agent"
  assume_role_policy = data.aws_iam_policy_document.ecs_agent.json
  managed_policy_arns = [
    "arn:aws:iam::aws:policy/service-role/AmazonEC2ContainerServiceforEC2Role",
    aws_iam_policy.password_policy_parameterstore.arn,
    aws_iam_policy.receptionist_table_policy.arn
  ]
}

resource "aws_iam_instance_profile" "ecs_agent" {
  name = "ecs-agent"
  role = aws_iam_role.ecs_agent.name
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


# ECS / EC2 Logs
resource "aws_cloudwatch_log_group" "receptionist_logs_group" {
  name = "receptionist_logs_group"

  tags = {
    project = "receptionist_bot"
  }
}


### VPC ################
resource "aws_vpc" "vpc" {
  cidr_block           = "10.0.0.0/16"
  enable_dns_support   = true
  enable_dns_hostnames = true
  tags = {
    Name    = "Terraform VPC"
    project = "receptionist_bot"
  }
}

resource "aws_internet_gateway" "internet_gateway" {
  vpc_id = aws_vpc.vpc.id
}

resource "aws_subnet" "pub_subnet" {
  vpc_id     = aws_vpc.vpc.id
  cidr_block = "10.0.0.0/24"
}

resource "aws_route_table" "public" {
  vpc_id = aws_vpc.vpc.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.internet_gateway.id
  }
}

resource "aws_route_table_association" "route_table_association" {
  subnet_id      = aws_subnet.pub_subnet.id
  route_table_id = aws_route_table.public.id
}


### SG ################
resource "aws_security_group" "ecs_sg" {
  vpc_id = aws_vpc.vpc.id

  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  ### Maybe do this?
  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  ### Maybe do this?Zc
  ingress {
    from_port   = 3000
    to_port     = 3000
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }


  egress {
    from_port   = 0
    to_port     = 65535
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
}
