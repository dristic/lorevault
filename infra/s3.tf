resource "random_id" "bucket_suffix" {
  byte_length = 4
}

resource "aws_s3_bucket" "blobs" {
  bucket = "lorevault-blobs-${var.environment}-${random_id.bucket_suffix.hex}"
  tags   = { Name = "lorevault-blobs" }
}

resource "aws_s3_bucket_server_side_encryption_configuration" "blobs" {
  bucket = aws_s3_bucket.blobs.id

  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "AES256"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "blobs" {
  bucket = aws_s3_bucket.blobs.id

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_s3_bucket_versioning" "blobs" {
  bucket = aws_s3_bucket.blobs.id
  versioning_configuration {
    status = "Disabled"
  }
}
