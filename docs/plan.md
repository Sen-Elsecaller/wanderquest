# Plan - hackathon "Find Your Way"

**Cierre:** 2026-09-30 22:00 UTC. **Meta:** top 3. **Red:** testnet.
**Alcance elegido:** profundidad on-chain sobre el flujo core, no ancho de pantallas.

## Qué existe el 30 de septiembre

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

## La restriccion que define el plan

Este contenedor **no alcanza la testnet**: `horizon-testnet.stellar.org`,
`soroban-testnet.stellar.org` y `friendbot.stellar.org` dan 403 en el proxy de
egress. Los tests unitarios de soroban-sdk corren en un host local sin red, asi
que compilar y testear se hace aca sin problema. **El deploy no.**

Dos salidas, en orden de preferencia:
- **A:** habilitar esos tres dominios en la politica de red del entorno. Elimina
  la dependencia y permite iterar contra la red sin bloquear a nadie.
- **B:** el deploy lo corre el usuario con un script de un comando (F2 lo entrega).

## Fases

### F0 - Desbloqueo (D1)
- `contracts/quest-manager/src/test.rs`: no existe y `lib.rs:187` lo declara, asi
  que el crate no compila. Se escribe con `ed25519-dalek` (ya esta en dev-deps)
  generando firmas reales.
- Migrar eventos de `env.events().publish()` (deprecado en 27.0.6) a `#[contractevent]`.
- Confirmar que `cargo build --target wasm32v1-none --release` produce el wasm.

**Listo cuando:** el workspace entero compila, todos los tests verdes, wasm generado.

### F1 - Contratos correctos (D2-D5)
- **Atar la firma al usuario.** Mensaje pasa de `quest_id || nonce` a
  `quest_id || nonce || user.to_xdr(env)`. Cierra el front-running: la prueba deja
  de ser al portador. (`to_xdr` verificado en el SDK; `to_payload` NO sirve, esta
  tras el feature `hazmat-address` y su doc desaconseja este uso.)
- **SEP-41 completo** en wq-token: falta `allowance`, `approve`, `transfer_from`,
  `burn_from`. Criterio de evaluacion directo ("uso claro y correcto de Stellar") y
  requisito para que wallets y explorers lo muestren bien. Ojo: en 27.0.6 el `to`
  de `transfer` es `MuxedAddress`, no `Address`.
- **TTL.** Nadie extiende el instance storage (`Admin`, `TotalSupply`, `Owner`,
  `Token`) ni las entradas `Quest`. Si el contrato se archiva, la demo muere.
- **Tests:** camino feliz; firma invalida; nonce reusado; prueba de otro usuario
  (el que demuestra el fix); quest inactiva; mint por no-admin; redeem completo con
  el arbol de auth y verificacion del 5% quemado.

**Listo cuando:** ambos contratos completos y la suite cubre los tres ataques.

### F2 - Deploy a testnet (D6-D7)
- `scripts/deploy-testnet.sh`: build optimizado, deploy de ambos contratos,
  `initialize` de cada uno, QuestManager como admin del token, y registro de 3
  quests reales de Santiago.
- Bloqueado por la restriccion de red (ver arriba).

**Listo cuando:** dos contract IDs vivos en testnet con quests registradas.

### F3 - Firmante y cableado (D8-D11)
- Adapter SSR de Astro (node).
- `POST /api/location/challenge` emite nonce con expiracion.
- `POST /api/location/sign` valida el nonce y firma `quest_id || nonce || address`.
- Llaves ed25519 por quest en variables de entorno. **Nunca en el repo.**
- `scan.astro`: leer QR -> challenge -> firma -> transaccion.
- `wallet.astro`: balance real leido del contrato.
- Wallet del usuario: keypair en el navegador (decision cerrada, demo).

**Listo cuando:** el recorrido completo funciona de punta a punta.

### F4 - Demo y entrega (D12-D14)
- Guion de demo, con los tres ataques como climax.
- Video. README para el juez. Submission.

**Listo cuando:** entregado antes del 30 a las 22:00 UTC.

## Riesgos

| Riesgo | Mitigacion |
|---|---|
| Deploy bloqueado por la red del entorno | Opcion A (allowlist) o script de un comando |
| El arbol de auth de `redeem` falla | Test en F1, antes de cablear nada |
| Disponibilidad del usuario para el deploy | Un solo comando, salida pegada de vuelta |
| Camara/QR en movil | Fallback de codigo escrito a mano |

## Preguntas abiertas

1. Habilitar los tres dominios de testnet en la politica de red del entorno? (opcion A)
2. Indentacion: preferencia del usuario es tabs; los contratos actuales usan 4
   espacios y rustfmt por defecto tambien. Se migra todo a tabs o se deja Rust como esta?
3. Las 3 quests de Santiago a registrar: cuales?
