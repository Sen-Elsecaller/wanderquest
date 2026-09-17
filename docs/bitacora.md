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

---

## 2026-09-16 (tarde) - Del contenedor al PC: F0, F1 y F2 en una sesion

**Cambio de entorno.** La sesion arranco en el PC del usuario (Windows), no en el
contenedor. Se invirtio la restriccion: la testnet **responde** (horizon, RPC y
friendbot), pero no habia toolchain. Instalado en la maquina: Rust 1.98.1 con host
`x86_64-pc-windows-gnu` y target `wasm32v1-none`, `stellar-cli` 28.0.0 por winget,
la doc oficial clonada en `~/stellar/stellar-docs`, y `@stellar/stellar-sdk` 17.

No hay MSVC Build Tools y el usuario pidio no instalar 6 GB de Visual Studio, asi
que el host es GNU. Su linker desborda el limite de 65535 ordinales al exportar el
`cdylib` de un contrato: `contracts/.cargo/config.toml` pasa
`-C link-arg=-Wl,--exclude-all-symbols` y con eso `cargo test` linkea. El wasm no
pasa por ahi.

### Bug serio, encontrado antes de desplegar

`quest-manager` dependia del crate `wq-token` solo para usar su cliente. Eso linkea
los `#[contractimpl]` del token dentro del wasm del manager: `quest_manager.wasm`
exportaba `mint`, `burn` y `transfer`, y los dos `initialize` (de distinta aridad)
colisionaban, asi que el linker **borro el del manager**. El contrato no se podia
inicializar; el deploy habria fallado en el primer comando.

Arreglo: declarar la interfaz con `#[contractclient]` y dejar `wq-token` como
`dev-dependency`. `quest_manager.wasm` paso de 18 exports a 10, los suyos.

**Leccion, ya en la skill:** despues de tocar dependencias entre contratos, mirar
los exports del wasm. El warning del linker era la unica pista y era facil de pasar
por alto entre el ruido del build.

### Hecho

- **F0:** `quest-manager/src/test.rs` escrito (11 tests con firmas ed25519 reales),
  eventos migrados a `#[contractevent]`, wasm limpio.
- **F1:** firma atada al usuario; SEP-41 completo implementando
  `token::TokenInterface` del SDK (allowances en storage temporal que vence junto
  con el permiso); TTL en instance, quests, nonces y balances. 22 tests verdes,
  clippy limpio.
- **F2:** desplegado en testnet. `scripts/deploy-testnet.ps1` lo rehace de cero.
  - WQToken `CC4LXDWGXJ4GSJNHLMM3RJOYSSK2TSQ5M7BFQMBVT7UDX63C34UJIDL5`
  - QuestManager `CABDQUAZM65KCFI5H7636HRZLSKOT27DUG6Y5IUPTHG6GD6YPSGHWTFL`
  - 3 quests registradas (5, 3 y 8 WQ), llaves de ubicacion en `.env`.
- **Prueba en vivo** (`scripts/smoke-testnet.mjs`): visita verificada acuña 5 WQ,
  canje deja 0.95 al comercio y quema 0.05, y los tres ataques se rechazan en la
  red - reuso con `WasmVm, InvalidAction`, prueba ajena y firma falsa con
  `Crypto, InvalidInput`.
- **Mitad de F3: el backend firmante.** Adapter `@astrojs/node` (la 11 pide Astro 7,
  este proyecto esta en Astro 5: va la 9.5), `POST /api/location/challenge` y
  `POST /api/location/sign` en `src/pages/api/`, con la logica en
  `src/lib/location-signer.ts`. Nonce de un solo uso, 5 minutos de vida, atado a su
  quest; la address la codifica el servidor, no el cliente. Las paginas siguen
  siendo estaticas: solo `/api` se renderiza on-demand.
  `scripts/smoke-signer.mjs` levanta el servidor construido, hace el handshake y
  acuña con esa firma contra testnet: **TODO OK**. No se toco ninguna pantalla.

### Verificado

- `Address.fromString(g).toScVal().toXDR()` del JS SDK produce **los mismos bytes**
  que `user.to_xdr(&env)` en el contrato. Lo prueba el mint en vivo.
- En los tests, `env.events().all()` solo trae los eventos de la ultima invocacion:
  leerlos antes de cualquier otra llamada.
- Una entrada `persistent` archivada no se puede recrear, solo restaurar
  (`state-archival.mdx`), asi que el registro de nonces sobrevive a su TTL.
- `instance().extend_ttl()` extiende instancia **y codigo**.

### Decisiones tomadas

- **Rust queda en 4 espacios con rustfmt por defecto.** La preferencia de tabs del
  usuario aplica al resto; forzarla en Rust pelea con toda la herramienta.
- **El deploy a testnet lo corre el agente**, ya que la red es alcanzable. Mainnet y
  fondos reales siguen siendo del usuario.
- Las 3 quests de Santiago se registraron sin decidir los lugares: el contrato solo
  guarda llave publica, recompensa y estado. El nombre y la foto son frontend.

### Trampa del bundle

`location-signer.ts` leia `.env` con una ruta relativa a `import.meta.url`. En
`astro dev` andaba; despues del build el modulo vive en `dist/server/` y la ruta
apunta a otro lado, asi que el firmante arrancaba sin ninguna llave y contestaba
404 a todo. Se resuelve desde `process.cwd()`. Lo agarro `smoke-signer.mjs`
justamente porque corre contra el build, no contra el dev server.

### Pendiente al cerrar

La otra mitad de F3: cablear `scan` y `wallet` a la cadena, y decidir como vive la
wallet del usuario en el browser. Es lo primero que toca pantallas, asi que espera
al usuario.
