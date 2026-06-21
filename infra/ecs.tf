resource "aws_ecs_cluster" "main" {
  name = "lorevault-${var.environment}"

  setting {
    name  = "containerInsights"
    value = "enabled"
  }

  tags = { Name = "lorevault-cluster" }
}

resource "aws_cloudwatch_log_group" "app" {
  name              = "/ecs/lorevault-${var.environment}"
  retention_in_days = 30
  tags              = { Name = "lorevault-logs" }
}

locals {
  image_uri = "${aws_ecr_repository.app.repository_url}:${var.image_tag}"

  redis_url = "redis://${aws_elasticache_cluster.main.cache_nodes[0].address}:6379"
  grpc_url  = "grpcs://${var.domain_name}:41337"
}

resource "aws_ecs_task_definition" "app" {
  family                   = "lorevault-${var.environment}"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = var.app_cpu
  memory                   = var.app_memory
  execution_role_arn       = aws_iam_role.ecs_execution.arn
  task_role_arn            = aws_iam_role.ecs_task.arn

  container_definitions = jsonencode([
    {
      name      = "lorevault"
      image     = local.image_uri
      essential = true

      portMappings = [
        { containerPort = 3000, protocol = "tcp", name = "rest" },
        { containerPort = 41337, protocol = "tcp", name = "grpc" },
      ]

      environment = [
        { name = "APP__SERVER__BIND",       value = "0.0.0.0:3000" },
        { name = "APP__SERVER__GRPC_BIND",  value = "0.0.0.0:41337" },
        { name = "APP__SERVER__PUBLIC_URL", value = local.grpc_url },
        { name = "APP__REDIS__URL",         value = local.redis_url },
        { name = "APP__STORAGE__S3_BUCKET", value = aws_s3_bucket.blobs.bucket },
        { name = "APP__STORAGE__S3_REGION", value = var.aws_region },
        # s3_endpoint is intentionally omitted — uses native AWS S3
      ]

      secrets = [
        {
          name      = "APP__DATABASE__URL"
          valueFrom = aws_secretsmanager_secret.db_url.arn
        },
        {
          name      = "APP__AUTH__JWT_SECRET"
          valueFrom = aws_secretsmanager_secret.jwt_secret.arn
        },
      ]

      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.app.name
          "awslogs-region"        = var.aws_region
          "awslogs-stream-prefix" = "ecs"
        }
      }

      healthCheck = {
        command     = ["CMD-SHELL", "curl -sf http://localhost:3000/healthz || exit 1"]
        interval    = 30
        timeout     = 5
        retries     = 3
        startPeriod = 60
      }
    }
  ])

  tags = { Name = "lorevault-task" }
}

resource "aws_ecs_service" "app" {
  name            = "lorevault-${var.environment}"
  cluster         = aws_ecs_cluster.main.id
  task_definition = aws_ecs_task_definition.app.arn
  desired_count   = var.app_desired_count
  launch_type     = "FARGATE"

  network_configuration {
    subnets          = aws_subnet.private[*].id
    security_groups  = [aws_security_group.app.id]
    assign_public_ip = false
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.rest.arn
    container_name   = "lorevault"
    container_port   = 3000
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.grpc.arn
    container_name   = "lorevault"
    container_port   = 41337
  }

  deployment_minimum_healthy_percent = 50
  deployment_maximum_percent         = 200

  deployment_circuit_breaker {
    enable   = true
    rollback = true
  }

  depends_on = [
    aws_lb_listener.https,
    aws_lb_listener.grpc,
    aws_iam_role_policy_attachment.ecs_execution_managed,
  ]

  tags = { Name = "lorevault-service" }

  lifecycle {
    ignore_changes = [desired_count]
  }
}
