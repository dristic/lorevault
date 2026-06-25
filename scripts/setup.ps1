param(
    [string]$Username = "dev",
    [string]$Email    = "dev@example.com",
    [string]$Password = "password",
    [string]$Repo     = "test-repo",
    [string]$Url      = "http://localhost:3000"
)

$ErrorActionPreference = 'Stop'
$base = $Url

function Post-Api($Path, $Body, $Token) {
    $headers = @{}
    if ($Token) { $headers['Authorization'] = "Bearer $Token" }
    Invoke-RestMethod -Method Post -Uri "$base$Path" `
        -ContentType 'application/json' `
        -Headers $headers `
        -Body ($Body | ConvertTo-Json -Compress)
}

Write-Host "Waiting for server..."
$ready = $false
for ($i = 0; $i -lt 30; $i++) {
    try { Invoke-RestMethod "$base/healthz" | Out-Null; $ready = $true; break } catch { Start-Sleep 1 }
}
if (-not $ready) { Write-Error "Server did not become ready after 30 seconds"; exit 1 }

Write-Host "Registering user (or logging in if already exists)..."
try {
    $auth = Post-Api "/api/v1/auth/register" @{ username = $Username; email = $Email; password = $Password }
} catch {
    $auth = Post-Api "/api/v1/auth/login" @{ login = $Username; password = $Password }
}

$jwt    = $auth.token
$userId = $auth.user_id

Write-Host "Creating API token..."
$tok = Post-Api "/api/v1/auth/tokens" @{ name = "dev-cli" } $jwt

Write-Host "Creating repository (skipping if already exists)..."
try {
    Post-Api "/api/v1/repos" @{
        owner_id   = $userId
        owner_type = "user"
        name       = $Repo
        visibility = "private"
    } $jwt | Out-Null
} catch {
    # 409 conflict means it already exists — that's fine
}

Write-Host ""
Write-Host "Done! Authenticate the Lore CLI:"
Write-Host ""
Write-Host "  lore auth login grpcs://localhost:41337 --token-type api-key --token $($tok.token)"
Write-Host ""
Write-Host "Or use the browser login flow:"
Write-Host ""
Write-Host "  lore auth login grpcs://localhost:41337"
