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

### Terraform State ############
resource "aws_s3_bucket" "terraform_state" {
  bucket = "terraform-receptionist-bot-state"
  # Enable versioning so we can see the full revision history of our state files
  versioning {
    enabled = true
  }
  # Enable server-side encryption by default
  server_side_encryption_configuration {
    rule {
      apply_server_side_encryption_by_default {
        sse_algorithm = "AES256"
      }
    }
  }
}
### Terraform State Lock #######
resource "aws_dynamodb_table" "terraform_locks" {
  name         = "terraform-receptionist-locks"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "LockID"
  attribute {
    name = "LockID"
    type = "S"
  }
}
