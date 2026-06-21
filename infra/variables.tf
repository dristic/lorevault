variable "aws_region" {
  description = "AWS region to deploy into"
  type        = string
  default     = "us-east-1"
}

variable "environment" {
  description = "Deployment environment (production, staging)"
  type        = string
  default     = "production"
}

variable "domain_name" {
  description = "Primary domain name (e.g. lorevault.example.com). An ACM cert will be looked up or created for this domain."
  type        = string
}

variable "acm_certificate_arn" {
  description = "ARN of an existing ACM certificate for domain_name. If empty, you must provision one separately and paste the ARN here."
  type        = string
  default     = ""
}

# ── Database ──────────────────────────────────────────────────────────────────

variable "db_instance_class" {
  description = "RDS instance type"
  type        = string
  default     = "db.t4g.small"
}

variable "db_allocated_storage" {
  description = "Initial RDS storage in GiB"
  type        = number
  default     = 20
}

variable "db_password" {
  description = "Master password for the RDS PostgreSQL instance"
  type        = string
  sensitive   = true
}

# ── Cache ─────────────────────────────────────────────────────────────────────

variable "redis_node_type" {
  description = "ElastiCache node type"
  type        = string
  default     = "cache.t4g.micro"
}

# ── Application ───────────────────────────────────────────────────────────────

variable "jwt_secret" {
  description = "Secret key used to sign JWTs"
  type        = string
  sensitive   = true
}

variable "image_tag" {
  description = "Docker image tag to deploy (ECR image tag)"
  type        = string
  default     = "latest"
}

variable "app_cpu" {
  description = "ECS task CPU units (1024 = 1 vCPU)"
  type        = number
  default     = 512
}

variable "app_memory" {
  description = "ECS task memory in MiB"
  type        = number
  default     = 1024
}

variable "app_desired_count" {
  description = "Number of ECS tasks to run"
  type        = number
  default     = 1
}

# ── Networking ────────────────────────────────────────────────────────────────

variable "vpc_cidr" {
  description = "CIDR block for the VPC"
  type        = string
  default     = "10.0.0.0/16"
}

variable "availability_zones" {
  description = "List of AZs to spread subnets across"
  type        = list(string)
  default     = ["us-east-1a", "us-east-1b"]
}
