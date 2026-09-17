# Plan - hackathon "Find Your Way"

**Cierre:** 2026-09-30 22:00 UTC. **Meta:** top 3. **Red:** testnet.
**Alcance elegido:** profundidad on-chain sobre el flujo core, no ancho de pantallas.

## Que existe el 30 de septiembre

1. **WQ no se puede falsificar.** El unico `admin` del token es el QuestManager.
   La unica puerta por la que entra WQ nuevo es una funcion que antes verifica
   on-chain una firma ed25519 de la ubicacion. Auditable leyendo el contrato.
2. **El recorrido completo se hace en vivo**, no en video editado: escanear ->
   firmar -> token en wallet -> gastar en comercio -> 5% quemado, con un explorer
   de Stellar al lado mostrando las transacciones.
3. **Tres ataques que fallan en vivo.** Prueba reusada: rechazada. Prueba de otro
   usuario: rechazada. Monto alterado: imposible, vive en el contrato.
   Este es el diferenciador frente a otros submissions.
4. **Dos pantallas reales, tres de maqueta.** `scan` y `wallet` contra cadena.
   `index`, `quest`, `profile` quedan como estan.
5. **Repo autoexplicativo.** README que un juez sigue para desplegar y correr.

**Fuera de alcance, declarado:** dinero real, passkeys, mapa real, comercios
reales, off-ramp a CLP. El GPS sigue siendo falsificable y se documenta como tal.

## Estado

F0, F1 y F2 estan hechas. Los puntos 1, 2 y 3 de arriba **ya ocurren contra
testnet**: `node scripts/smoke-testnet.mjs` los corre de punta a punta y sale
`TODO OK`. Lo que falta es que eso pase desde la app en vez de desde un script.

### F0 - Desbloqueo — **hecho**
- `contracts/quest-manager/src/test.rs` escrito con firmas ed25519 reales.
- Eventos migrados de `env.events().publish()` a `#[contractevent]`.
- Bug encontrado y corregido: depender del crate `wq-token` metia los exports del
  token en el wasm del manager y borraba su `initialize`. Ver la skill.

### F1 - Contratos correctos — **hecho**
- Firma atada al usuario: `quest_id || nonce || user.to_xdr(env)`.
- SEP-41 completo implementando `token::TokenInterface` del SDK.
- TTL: instance (instancia + codigo), quests, nonces usados y balances.
- 22 tests: camino feliz, los tres ataques, quest inactiva, owner, allowances
  (gastada, excedida, vencida, revocada), muxed, y el arbol de auth de `redeem`.

### F2 - Deploy a testnet — **hecho**
- `scripts/deploy-testnet.ps1`: build, deploy de ambos, cableado y registro de las
  3 quests, de una sola corrida.
- `scripts/new-location-keys.mjs` genera las llaves ed25519 de ubicacion; los
  secretos quedan en `.env` (gitignoreado), las publicas van on-chain.
- IDs en `deployments/testnet.json`. Verificado leyendo el estado de la red.

### F3 - Firmante y cableado — **backend hecho, falta la app**

Hecho, sin tocar ninguna pantalla:
- Adapter `@astrojs/node`. Las paginas siguen prerenderizadas; solo `/api` corre
  en el servidor.
- `POST /api/location/challenge` emite un nonce de un solo uso, 5 min de vida,
  atado a su quest.
- `POST /api/location/sign` valida el nonce y firma `quest_id || nonce || address`.
  La address la codifica el servidor.
- Llaves ed25519 por quest desde `.env`. **Nunca en el repo.**
- `scripts/smoke-signer.mjs` levanta el build, hace el handshake y acuña con esa
  firma contra testnet.

Falta:
- `scan.astro`: leer QR -> challenge -> firma -> transaccion.
- `wallet.astro`: balance real leido del contrato.
- Wallet del usuario: keypair en el navegador (decision cerrada, demo); falta
  definir donde vive (ver pregunta 2).

**Listo cuando:** el recorrido completo funciona desde la app, no desde un script.

### F4 - Demo y entrega
- Guion de demo, con los tres ataques como climax.
- Video. README para el juez. Submission.

**Listo cuando:** entregado antes del 30 a las 22:00 UTC.

## Riesgos

| Riesgo | Mitigacion |
|---|---|
| El arbol de auth de `redeem` falla desde el browser | Ya funciona con la cuenta del usuario como source; replicar eso en la app |
| Camara/QR en movil | Fallback de codigo escrito a mano |
| Contratos archivados antes de la demo | TTL de ~30 dias y se extiende en cada escritura |
| Una llave de ubicacion se filtra | Es demo y estan en `.env`; regenerar con `new-location-keys.mjs` y re-registrar |

## Preguntas abiertas

1. Las 3 quests de Santiago: cuales? Solo afecta nombre, foto y coordenadas en el
   frontend - la cadena ya tiene las 3 registradas (5, 3 y 8 WQ).
2. La wallet del usuario en el browser: keypair generado y guardado en
   `localStorage`, o pedir que peguen una secreta de testnet? Lo primero es mas
   demostrable; lo segundo evita hablar de custodia.
