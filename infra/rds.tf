resource "aws_db_subnet_group" "main" {
  name       = "lorevault-${var.environment}"
  subnet_ids = aws_subnet.private[*].id
  tags       = { Name = "lorevault-db-subnet-group" }
}

resource "aws_db_instance" "main" {
  identifier        = "lorevault-${var.environment}"
  engine            = "postgres"
  engine_version    = "16"
  instance_class    = var.db_instance_class
  allocated_storage = var.db_allocated_storage
  storage_encrypted = true
  storage_type      = "gp3"

  db_name  = "lorevault"
  username = "lorevault"
  password = var.db_password

  db_subnet_group_name   = aws_db_subnet_group.main.name
  vpc_security_group_ids = [aws_security_group.rds.id]
  publicly_accessible    = false

  backup_retention_period = 7
  skip_final_snapshot     = false
  final_snapshot_identifier = "lorevault-${var.environment}-final-${formatdate("YYYY-MM-DD", timestamp())}"

  deletion_protection = true

  tags = { Name = "lorevault-postgres" }

  lifecycle {
    ignore_changes = [final_snapshot_identifier]
  }
}

# Store the DB URL in Secrets Manager so ECS can inject it without exposing
# the password in plaintext task definition environment variables.
resource "aws_secretsmanager_secret" "db_url" {
  name                    = "lorevault/${var.environment}/database-url"
  recovery_window_in_days = 7
}

resource "aws_secretsmanager_secret_version" "db_url" {
  secret_id     = aws_secretsmanager_secret.db_url.id
  secret_string = "postgres://${aws_db_instance.main.username}:${var.db_password}@${aws_db_instance.main.endpoint}/${aws_db_instance.main.db_name}"
}
