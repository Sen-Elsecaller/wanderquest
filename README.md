# WanderQuest

Plataforma de exploración urbana gamificada sobre **Stellar**. Completás quests físicas, ganás tokens **WQ** canjeables en comercios locales, y el comercio paga solo por **visitas reales verificadas** (CPVV — cost per verified visit).

**Hackathon "Find Your Way" (Stellar Passport) — General Track. Todo en testnet.**

![Stellar](https://img.shields.io/badge/Powered%20by-Stellar-black?logo=stellar)
![Astro](https://img.shields.io/badge/Built%20with-Astro-ff5d01?logo=astro)

## La idea en una línea

Un token no se puede acuñar porque alguien diga que estuvo en un lugar: se acuña porque el contrato **verifica on-chain una firma ed25519 emitida en ese lugar**, atada a quien la reclama.

## Contratos desplegados (testnet)

| Contrato | ID |
|---|---|
| WQToken | [`CC4LXDWGXJ4GSJNHLMM3RJOYSSK2TSQ5M7BFQMBVT7UDX63C34UJIDL5`](https://stellar.expert/explorer/testnet/contract/CC4LXDWGXJ4GSJNHLMM3RJOYSSK2TSQ5M7BFQMBVT7UDX63C34UJIDL5) |
| QuestManager | [`CABDQUAZM65KCFI5H7636HRZLSKOT27DUG6Y5IUPTHG6GD6YPSGHWTFL`](https://stellar.expert/explorer/testnet/contract/CABDQUAZM65KCFI5H7636HRZLSKOT27DUG6Y5IUPTHG6GD6YPSGHWTFL) |

- **WQToken** — fungible, 7 decimales, SEP-41 completo. Su único `admin` es el QuestManager, así que **WQ nuevo solo nace de una visita verificada**.
- **QuestManager** — guarda la clave pública ed25519 de cada quest. `complete_quest` verifica la firma con `env.crypto().ed25519_verify` antes de acuñar, y marca el nonce usado. `redeem` paga 95% al comercio y quema 5%.

### Cómo se prueba una visita

El backend firmante de cada ubicación firma `quest_id || nonce || address_del_usuario`. La clave secreta nunca está en el QR ni en un tag pasivo — quien la tuviera podría acuñar desde su casa y el CPVV se cae. Meter la address del usuario adentro del mensaje es lo que impide que la prueba sea al portador.

**Limitación honesta:** el GPS es falsificable. Lo que el sistema prueba es posesión de una firma emitida por el firmante del local.

## Correrlo

```bash
npm install
npm run dev              # landing + prototipo de app
```

Contratos (necesita Rust con el target `wasm32v1-none`):

```bash
cd contracts
cargo test                                      # 22 tests
cargo build --target wasm32v1-none --release
```

Contra la red (necesita [stellar-cli](https://developers.stellar.org/docs/tools/cli)):

```powershell
node scripts/new-location-keys.mjs 3    # llaves de ubicación -> .env (nunca se commitea)
pwsh scripts/deploy-testnet.ps1         # deploy + cableado + registro de quests
node scripts/smoke-testnet.mjs          # el recorrido completo, en vivo
```

`smoke-testnet.mjs` hace la visita verificada, el canje con su quema, y después intenta los tres ataques que tienen que fallar: prueba reusada, prueba emitida para otro usuario, y firma de otra llave.

## Stack

| Capa | Tecnología |
|---|---|
| Frontend | Astro 5 · Tailwind CSS v4 · GSAP/ScrollTrigger · Vanta.js |
| Contratos | Rust · soroban-sdk 27.0.6 · target `wasm32v1-none` |
| Cadena (JS) | `@stellar/stellar-sdk` 17 · stellar-cli 28 |

## Rutas

| Ruta | Descripción |
|------|-------------|
| `/` | Landing page |
| `/app` | Mapa con quests cercanas |
| `/app/quest` | Detalle de quest |
| `/app/scan` | Scanner QR |
| `/app/wallet` | Billetera WQ |
| `/app/profile` | Perfil de usuario |

## Modelo económico

- **1 WQ ≈ 100 CLP.** Tokens fungibles, canjeables en cualquier comercio aliado.
- Fees: depósito de comercio 10%, recirculación 5% (se quema), retiro de usuario 20%, canje 0% para el usuario.
- Números provisionales hasta tener algo sólido y jugable.

---

*Powered by Stellar*
