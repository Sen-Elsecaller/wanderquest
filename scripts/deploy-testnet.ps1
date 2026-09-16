<#
.SYNOPSIS
    Deploys WQToken and QuestManager to Stellar testnet and wires them together.

.DESCRIPTION
    One command, start to finish: builds the Wasm, deploys both contracts,
    makes QuestManager the token's only minter, and registers the quests whose
    signing keys live in .env.

        pwsh scripts/deploy-testnet.ps1

    Everything it writes goes to deployments/testnet.json, which is safe to
    commit - contract ids and location public keys are public by nature. The
    location secrets stay in .env and are never read out of this machine.

    Re-running deploys a NEW pair of contracts; pass -Force to overwrite the
    recorded deployment.

.PARAMETER Identity
    Name of the stellar-cli identity that owns the contracts. Created and funded
    from friendbot if it does not exist yet.
#>
param(
    [string]$Identity = "wanderquest-admin",
    [string]$Network = "testnet",
    [switch]$Force
)

$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $PSScriptRoot
$deploymentFile = Join-Path $repo "deployments\$Network.json"

# Reward per quest, in stroops (7 decimals). 1 WQ = 10_000_000 = ~100 CLP.
$rewards = @{ 1 = 50000000; 2 = 30000000; 3 = 80000000 }

function Step($message) {
    Write-Host ""
    Write-Host "==> $message" -ForegroundColor Cyan
}

function Invoke-Stellar($stellarArgs) {
    $output = & stellar @stellarArgs
    if ($LASTEXITCODE -ne 0) {
        throw "stellar $($stellarArgs -join ' ') fallo con codigo $LASTEXITCODE"
    }
    return ($output | Select-Object -Last 1).ToString().Trim()
}

if (-not (Get-Command stellar -ErrorAction SilentlyContinue)) {
    throw "stellar-cli no esta en el PATH. Instalar con: winget install --id Stellar.StellarCLI"
}
if ((Test-Path $deploymentFile) -and (-not $Force)) {
    throw "Ya hay un deploy registrado en $deploymentFile. Usar -Force para reemplazarlo."
}

Step "Identidad $Identity"
$identities = & stellar keys ls
if ($identities -contains $Identity) {
    Write-Host "    ya existe"
} else {
    & stellar keys generate $Identity --network $Network --fund | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "no se pudo generar la identidad" }
    Write-Host "    creada y fondeada por friendbot"
}
$adminAddress = Invoke-Stellar @("keys", "public-key", $Identity)
Write-Host "    $adminAddress"

Step "Build de los contratos"
Push-Location (Join-Path $repo "contracts")
try {
    & cargo build --target wasm32v1-none --release
    if ($LASTEXITCODE -ne 0) { throw "el build de wasm fallo" }
} finally {
    Pop-Location
}
$wasmDir = Join-Path $repo "contracts\target\wasm32v1-none\release"

Step "Deploy de WQToken"
$tokenId = Invoke-Stellar @(
    "contract", "deploy",
    "--wasm", (Join-Path $wasmDir "wq_token.wasm"),
    "--source-account", $Identity,
    "--network", $Network
)
Write-Host "    $tokenId"

Step "Deploy de QuestManager"
$managerId = Invoke-Stellar @(
    "contract", "deploy",
    "--wasm", (Join-Path $wasmDir "quest_manager.wasm"),
    "--source-account", $Identity,
    "--network", $Network
)
Write-Host "    $managerId"

Step "QuestManager queda como unico admin del token"
Invoke-Stellar @(
    "contract", "invoke", "--id", $tokenId,
    "--source-account", $Identity, "--network", $Network,
    "--", "initialize", "--admin", $managerId
) | Out-Null

Step "initialize de QuestManager"
Invoke-Stellar @(
    "contract", "invoke", "--id", $managerId,
    "--source-account", $Identity, "--network", $Network,
    "--", "initialize", "--owner", $adminAddress, "--token", $tokenId
) | Out-Null

Step "Registro de quests"
$envFile = Join-Path $repo ".env"
if (-not (Test-Path $envFile)) {
    throw "falta .env con las llaves de ubicacion. Generarlas con: node scripts/new-location-keys.mjs 3"
}

$quests = @()
foreach ($line in Get-Content $envFile) {
    if ($line -notmatch '^WQ_LOCATION_SECRET_(\d+)=([0-9a-fA-F]{64})$') { continue }
    $questId = [int]$Matches[1]
    $publicKey = & node (Join-Path $repo "scripts\location-pubkey.mjs") $Matches[2]
    if ($LASTEXITCODE -ne 0) { throw "no se pudo derivar la llave publica de la quest $questId" }

    $reward = $rewards[$questId]
    if (-not $reward) { $reward = 10000000 }

    Invoke-Stellar @(
        "contract", "invoke", "--id", $managerId,
        "--source-account", $Identity, "--network", $Network,
        "--", "register_quest",
        "--quest_id", $questId,
        "--location_pubkey", $publicKey,
        "--reward", $reward
    ) | Out-Null

    Write-Host "    quest $questId  reward $($reward / 10000000) WQ  pubkey $($publicKey.Substring(0, 16))..."
    $quests += [ordered]@{ quest_id = $questId; location_pubkey = $publicKey; reward = $reward }
}
if ($quests.Count -eq 0) { throw "no habia ninguna llave de ubicacion en .env" }

Step "Registro del deploy"
New-Item -ItemType Directory -Force (Split-Path $deploymentFile) | Out-Null
$record = [ordered]@{
    network = $Network
    deployed_at = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    admin_address = $adminAddress
    identity = $Identity
    wq_token = $tokenId
    quest_manager = $managerId
    quests = $quests
}
# WriteAllText, not Set-Content: Windows PowerShell writes a BOM with -Encoding
# utf8, and JSON.parse chokes on it.
[System.IO.File]::WriteAllText($deploymentFile, ($record | ConvertTo-Json -Depth 5))
Write-Host "    $deploymentFile"

Write-Host ""
Write-Host "Listo." -ForegroundColor Green
Write-Host "  WQToken       $tokenId"
Write-Host "  QuestManager  $managerId"
Write-Host "  Explorer      https://stellar.expert/explorer/testnet/contract/$managerId"
