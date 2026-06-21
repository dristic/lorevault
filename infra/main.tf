terraform {
  required_version = ">= 1.6"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.0"
    }
  }

  # Remote state — create this bucket manually before first apply,
  # or remove this block and use local state for initial bootstrapping.
  backend "s3" {
    bucket         = "lorevault-tfstate"
    key            = "lorevault/terraform.tfstate"
    region         = "us-east-1"
    encrypt        = true
    dynamodb_table = "lorevault-tflock"
  }
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = "lorevault"
      Environment = var.environment
      ManagedBy   = "opentofu"
    }
  }
}

provider "random" {}
