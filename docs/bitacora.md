# Bitacora

Registro append-only de sesiones: que se decidio, que se verifico, que cambio.
**Excepcion deliberada** a la regla de `.md` del `CLAUDE.md` ("referencia viva, no
bitacora"): ese estilo aplica a los docs de referencia. Este archivo es historico -
se agrega al final, no se reescribe.

---

## 2026-09-16 - Retomar contexto, auditoria y plan

**Estado al empezar:** la sesion arranco en frio. `master` tenia un solo commit
(landing + prototipo) y el `CLAUDE.md` de marzo. El trabajo de la sesion anterior
estaba en la rama `feat/onchain-contracts`, sin mergear.

### Hecho

- Merge fast-forward de `feat/onchain-contracts` a `master` (`35d7ffb`). Ahi venian
  los dos contratos Soroban y el `CLAUDE.md` con el objetivo nuevo.
- **Primera compilacion real del workspace.** Resultados:
  - `wq-token` compila; sus 4 tests pasan.
  - `quest-manager` **no compila**: `error[E0583]`, `lib.rs:187` declara `mod test;`
    y `src/test.rs` no existe.
  - `soroban-sdk` resuelve a **27.0.6**. `env.events().publish()` esta **deprecado**;
    la API actual es el macro `#[contractevent]`.
- Commit de `contracts/Cargo.lock` (fija 27.0.6 y el arbol transitivo, para que el
  build sea reproducible entre el contenedor y el PC del usuario) y de los
  `test_snapshots` que genera el SDK (`773a442`).
- Instalado el target `wasm32v1-none`.
- Clonada la documentacion oficial completa en `/home/user/stellar/stellar-docs`
  (991 archivos .md/.mdx, commit `0efbb05`).

### Verificado contra el SDK y la doc oficial

- `#[contractevent]` existe (`soroban-sdk/src/lib.rs:1056`).
- `Address::to_payload()` **no sirve** para meter la address en un mensaje firmado:
  esta tras `#[cfg(feature = "hazmat-address")]` y su propia doc desaconseja usarlo
  para verificacion de firmas.
- La via correcta es `ToXdr::to_xdr(self, env) -> Bytes` (`src/xdr.rs:48`), con impl
  blanket para todo `T: IntoVal<Env, Val>`, o sea aplica a `Address`.
- SEP-41 (`docs/tokens/token-interface.mdx`) exige `allowance`, `approve`,
  `transfer_from`, `burn_from`; wq-token no los tiene. En 27.0.6 el `to` de
  `transfer` es `MuxedAddress`.

### Hallazgos de auditoria

- **Front-running real.** La firma iba sobre `quest_id || nonce`, sin decir para
  quien. Cualquiera que la viera podia cobrarla. El ataque practico no es sofisticado:
  compartir la firma por mensajeria y que gane el primero en subirla.
- **La llave de ubicacion.** El docstring ofrecia tres arquitecturas (llave en el QR /
  en tag NFC / en backend). Las dos primeras exponen la llave secreta: quien fotografia
  el QR acuña desde su casa para siempre. Eso no es una limitacion de MVP, rompe el CPVV.
- Instance storage sin bump de TTL en ambos contratos.
- `redeem` necesita que la firma del usuario cubra tres nodos del arbol de auth
  (`redeem`, `transfer`, `burn`). Punto tipico de falla.

### Restriccion de entorno descubierta

El contenedor **no alcanza la testnet**: 403 en `horizon-testnet.stellar.org`,
`soroban-testnet.stellar.org` y `friendbot.stellar.org`. Tambien bloqueados
`developers.stellar.org` (resuelto clonando la doc), `stellarpassport.xyz`,
`dorahacks.io` y el MCP `stellar-raven` (`raven.stellar.buzz`). Compilar y testear
se hace aca; **desplegar no**.

### Decisiones tomadas

- **Meta:** top 3 del hackathon al 30 de septiembre. No se optimiza para escalar
  despues; si va bien, se escala despues.
- **Arquitectura de verificacion: challenge-response con backend firmante.** El QR
  lleva `quest_id` + nonce rotativo; la llave nunca sale del servidor; se firma
  incluyendo la address del usuario. Resuelve exposicion de llave, replay e
  intransferibilidad de una vez.
- **Se aplica el fix de front-running** (consecuencia de lo anterior).
- **Modelo de trabajo:** el agente escribe, compila, testea e itera en el contenedor.
  El usuario revisa y corre el deploy. Reemplaza la regla previa de "no compilo yo".
- **Rama:** se trabaja directo sobre `master`.
- Regla de reutilizacion del hackathon: **descartada como riesgo** por el usuario.
  Lo que se reusa es el concepto; el valor esta en llevarlo a cabo.

### Pendiente al cerrar

F0 del plan (`docs/plan.md`): escribir `quest-manager/src/test.rs` y dejar el
workspace compilando entero.
