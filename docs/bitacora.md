# Bitácora — WanderQuest

Registro de decisiones, hallazgos y contexto de sesión — orden **reverso-cronológico**
(más reciente arriba), agrupado por semana. Acá va lo específico de cada sesión; el estado
vivo del proyecto está en `CLAUDE.md`, el plan en [plan.md](plan.md), y cómo se verifica
cada API de Soroban en la skill `.claude/skills/stellar-soroban/`.

**Al inicio de sesión:** leer sólo las últimas 2 o 3 sesiones (las primeras del archivo).
Para historial más viejo, buscar por texto o por tag.

**Tags:** revisar esta lista antes de crear un tag nuevo — evitar sinónimos. Formato:
kebab-case, sin tildes, a nivel sistema/módulo.

`#contracts` `#token` `#quest-manager` `#sep-41` `#security` `#deploy` `#testnet`
`#signer` `#frontend` `#tooling` `#docs` `#process-rules`

**La fecha es la del día, y punto.** No se deduce de la última entrada ni se incrementa. Si
ya hay una entrada de hoy, la nueva es `(Sesión 2)`, `(Sesión 3)` en el título. La semana se
titula con su rango de fechas, no con un número de semana del proyecto: hubo seis meses entre
el commit inicial y el trabajo real, y numerar desde marzo mentiría sobre el ritmo.

**Si la fecha de hoy contradice lo que dice la última entrada, la equivocada es la entrada:**
corregirla, no acomodar la nueva alrededor. `git log` la verifica.

---

## Semana 14 - 20 de septiembre

### Sesión: 2026-09-16 (Sesión 2) — Del contenedor al PC, y los contratos a la red

**Tags:** #contracts #token #quest-manager #sep-41 #security #deploy #testnet #signer #tooling #docs

La sesión arrancó en el PC del usuario, no en el contenedor, y eso **dio vuelta la
restricción que definía el plan entero**: la testnet responde, pero no había con qué compilar.
Terminó con F0, F1 y F2 cerradas, los dos contratos vivos en testnet, y el recorrido completo
—acuñar, canjear, y los tres ataques fallando— ocurriendo contra la red de verdad.

El usuario pidió explícitamente trabajar solo: *"instala lo que tengas que instalar para yo
hacer lo menos posible"*, *"ordena el proyecto como veas mejor"*.

#### El entorno se dio vuelta

| | Contenedor | PC del usuario |
|---|---|---|
| horizon / RPC / friendbot | 403 | **responden** |
| `cargo`, target wasm | listos | **no existían** |
| `stellar-cli` | por instalar | **no existía** |
| doc oficial, SDK vendorizado | en disco | **no estaban** |

Instalado: Rust 1.98.1, `stellar-cli` 28.0.0 por winget, la doc oficial clonada, y del lado
JS `@stellar/stellar-sdk` 17.

**El host es el toolchain GNU y no MSVC**, porque el usuario pidió no bajarse Visual Studio
Build Tools. El costo aparece enseguida: un contrato Soroban se compila también como `cdylib`
para el host durante `cargo test`, el linker de mingw exporta todo símbolo público y
**desborda su límite de 65535 ordinales** con este árbol de dependencias. La salida es una
línea en `contracts/.cargo/config.toml`:

```toml
rustflags = ["-C", "link-arg=-Wl,--exclude-all-symbols"]
```

Nadie carga esa DLL —los tests usan el `rlib`—, así que no exportar nada es gratis. El wasm
ni se entera: usa `rust-lld`.

#### El bug que habría roto el deploy en el primer comando

F0 terminó, los tests en verde, y el build a wasm tiró **un warning entre cuarenta líneas de
ruido**: `function signature mismatch: initialize`. Mirar los exports del wasm en vez de
seguir de largo fue lo que salvó la sesión:

```
quest_manager (18 exports): _, admin, balance, burn, complete_quest, decimals,
get_quest, memory, mint, name, owner, redeem, register_quest, set_quest_active,
symbol, token, total_supply, transfer
```

Dos cosas mal, y la segunda es fatal: el manager exportaba `mint`, `burn` y `transfer` —que
son del token—, y **`initialize` no estaba**. El deploy habría fallado en el primer comando
después de subir el wasm.

La causa: `quest-manager` tenía `wq-token = { path = "../wq-token" }` en `[dependencies]`
sólo para usar `WQTokenClient`. Eso linkea los `#[contractimpl]` del token dentro del wasm
del manager; los dos `initialize` tienen distinta aridad, colisionan, y el linker se come
uno.

La forma correcta de llamar a otro contrato es **no depender de su crate**, sino declarar la
interfaz que se le va a pedir:

```rust
#[contractclient(name = "TokenClient")]
pub trait TokenInterface {
    fn mint(env: Env, to: Address, amount: i128);
}
```

`wq-token` quedó como `dev-dependency` para los tests. El wasm bajó de 18 exports a sus 10
legítimos y de 22.804 a 16.528 bytes. **La regla quedó en la skill: después de tocar
dependencias entre contratos, mirar los exports.**

#### SEP-41: implementar el trait del SDK en vez de copiarlo

`soroban_sdk::token::TokenInterface` es un `#[contracttrait]`. Implementarlo hace que las
firmas las garantice el SDK y no un copiado a mano de la doc — que era justo el riesgo, porque
en 27.0.6 el `to` de `transfer` es `MuxedAddress` y eso es fácil de pasar por alto.

Lo que no es parte de la interfaz —`initialize`, `mint`, `admin`, `total_supply`— vive en un
bloque aparte. Los dos `#[contractimpl]` alimentan el mismo `WQTokenClient`, así que desde
afuera se ve un solo contrato.

Las allowances van en storage **temporal**, con el TTL de la entrada igual al vencimiento del
permiso: la entrada muere sola cuando el permiso muere. Y una allowance vencida se lee como
cero aunque el host todavía no la haya archivado, que es lo que evita que el borde dependa de
cuándo pasa el recolector.

#### El TTL, que es lo que decide si la demo existe el día 30

Ninguno de los dos contratos extendía su `instance` storage. Un contrato archivado **deja de
responder**, así que la demo se moría sola con el tiempo. Ahora cada función que escribe
extiende la instancia —que cubre instancia y código—, las quests y los nonces usados extienden
su entrada, y los balances ya lo hacían.

Un detalle que vale por sí solo: **una entrada `persistent` archivada no se puede recrear,
sólo restaurar** (`state-archival.mdx`). O sea que el registro de nonces gastados sobrevive a
su propio TTL: el anti-replay no depende del bump, el bump sólo le ahorra a alguien el trámite
de restaurar.

#### El deploy, y la prueba que importa

`scripts/deploy-testnet.ps1` hace todo de una corrida: build, deploy de los dos, QuestManager
como único admin del token, y registro de las tres quests cuyas llaves de firma salen de
`.env`.

- WQToken `CC4LXDWGXJ4GSJNHLMM3RJOYSSK2TSQ5M7BFQMBVT7UDX63C34UJIDL5`
- QuestManager `CABDQUAZM65KCFI5H7636HRZLSKOT27DUG6Y5IUPTHG6GD6YPSGHWTFL`

Pero el deploy no prueba nada por sí solo. `scripts/smoke-testnet.mjs` camina el recorrido
entero con una cuenta descartable y después **intenta los tres ataques**:

| | Resultado en la red |
|---|---|
| Visita verificada | acuña 5 WQ |
| Canje de 1 WQ | 0.95 al comercio, 0.05 quemado |
| Prueba reusada | `Error(WasmVm, InvalidAction)` |
| Prueba emitida para otro usuario | `Error(Crypto, InvalidInput)` |
| Firma de otra llave | `Error(Crypto, InvalidInput)` |

De paso confirmó lo que F3 necesitaba y nadie había verificado: **el XDR que produce
`Address.fromString(g).toScVal().toXDR()` en el JS SDK es byte a byte el que
`user.to_xdr(&env)` reconstruye adentro del contrato**. Si no coincidieran, el backend
firmante firmaría algo que la cadena no reconoce, y el síntoma sería una firma inválida sin
ninguna pista de por qué.

#### El backend firmante, y la trampa del bundle

Con tiempo de sobra se hizo la mitad de F3 que no toca ninguna pantalla: adapter
`@astrojs/node` —la 11 pide Astro 7 y el proyecto está en Astro 5, así que va la 9.5— y dos
rutas, `POST /api/location/challenge` y `POST /api/location/sign`. Las páginas siguen
prerenderizadas; sólo `/api` corre en el servidor.

Dos decisiones chicas que importan: **el nonce se gasta antes de firmar** —una prueba se emite
una sola vez, pase lo que pase después— y **la address la codifica el servidor**, no el
cliente, que es exactamente lo que ata la prueba a quien la reclama.

El bug de la parte: `location-signer.ts` leía `.env` con una ruta relativa a
`import.meta.url`. Anda en `astro dev` y **se rompe después del build**, porque el módulo
termina bundleado en `dist/server/` y la ruta apunta a otro lado. El firmante arrancaba sin
ninguna llave y contestaba 404 a todo. Se resuelve desde `process.cwd()`. Lo agarró
`smoke-signer.mjs` justamente porque corre contra el build y no contra el dev server — si el
smoke test hubiera usado `astro dev`, el bug viajaba hasta la demo.

#### Decisiones tomadas

- **Rust queda en 4 espacios con rustfmt por defecto.** La preferencia de tabs del usuario
  aplica al resto del proyecto; forzarla en Rust pelea con toda la herramienta.
- **El deploy a testnet lo corre el agente.** La regla anterior decía "el deploy es del
  usuario *mientras la testnet siga bloqueada acá*", y dejó de estarlo. Mainnet y fondos
  reales siguen siendo del usuario.
- **Las tres quests se registraron sin decidir los lugares.** El contrato sólo guarda llave
  pública, recompensa y estado: el nombre, la foto y las coordenadas son frontend. Quedaron
  con 5, 3 y 8 WQ.
- **La bitácora pasa a formato reverso-cronológico**, el mismo de Little Big Potions.

#### Estado

22 tests unitarios verdes, clippy limpio, los dos wasm generados, los contratos desplegados y
los dos smoke tests contra testnet en `TODO OK`. Cuatro commits pusheados a `master`
(`4a5d4b7..d586f7e`).

Falta la otra mitad de F3 —cablear `scan` y `wallet`— y ahí hay **dos preguntas para el
usuario**: los tres lugares de Santiago, y dónde vive la wallet del usuario en el browser
(keypair en `localStorage`, o pegar una secreta de testnet). Las dos están al final de
[plan.md](plan.md).

---

### Sesión: 2026-09-16 — Retomar contexto, auditoría y plan

**Tags:** #contracts #security #docs #process-rules

La sesión arrancó en frío: `master` tenía un solo commit —landing y prototipo— y el
`CLAUDE.md` de marzo. El trabajo de la sesión anterior estaba en `feat/onchain-contracts`, sin
mergear.

#### Primera compilación real del workspace

Merge fast-forward de la rama a `master` (`35d7ffb`), y recién ahí se supo qué había:

- `wq-token` compila; sus 4 tests pasan.
- `quest-manager` **no compila**: `error[E0583]`, `lib.rs:187` declara `mod test;` y
  `src/test.rs` no existe.
- `soroban-sdk` resuelve a **27.0.6**, y `env.events().publish()` está **deprecado**.

Se commiteó `contracts/Cargo.lock` a propósito —fija 27.0.6 y todo el árbol transitivo, para
que el build sea reproducible entre máquinas— junto con los `test_snapshots` que genera el SDK
(`773a442`). Instalado el target `wasm32v1-none` y clonada la documentación oficial completa
(991 archivos `.md`/`.mdx`).

#### Verificado contra el SDK y la doc, no de memoria

- `#[contractevent]` existe (`soroban-sdk/src/lib.rs:1056`) y es la API actual de eventos.
- `Address::to_payload()` **no sirve** para meter la address en un mensaje firmado: está tras
  `#[cfg(feature = "hazmat-address")]` y su propia doc desaconseja usarlo para verificación de
  firmas.
- La vía correcta es `ToXdr::to_xdr(self, env) -> Bytes` (`src/xdr.rs:48`), con impl blanket
  para todo `T: IntoVal<Env, Val>`, o sea aplica a `Address`.
- SEP-41 (`docs/tokens/token-interface.mdx`) exige `allowance`, `approve`, `transfer_from` y
  `burn_from`, que wq-token no tenía. En 27.0.6 el `to` de `transfer` es `MuxedAddress`.

#### Hallazgos de auditoría

- **Front-running real.** La firma iba sobre `quest_id || nonce`, sin decir para quién.
  Cualquiera que la viera podía cobrarla, y el ataque práctico no es sofisticado: compartir la
  firma por mensajería y que gane el primero en subirla.
- **La llave de ubicación.** El docstring ofrecía tres arquitecturas —llave en el QR, en tag
  NFC, o en backend—. Las dos primeras exponen la llave secreta: quien fotografía el QR acuña
  desde su casa para siempre. Eso no es una limitación de MVP, rompe el CPVV entero.
- Instance storage sin bump de TTL en los dos contratos.
- `redeem` necesita que la firma del usuario cubra tres nodos del árbol de auth (`redeem`,
  `transfer`, `burn`). Punto típico de falla.

#### La restricción que definió el plan

El contenedor **no alcanzaba la testnet**: 403 en `horizon-testnet.stellar.org`,
`soroban-testnet.stellar.org` y `friendbot.stellar.org`. También bloqueados
`developers.stellar.org` —resuelto clonando la doc—, `stellarpassport.xyz`, `dorahacks.io` y
el MCP `stellar-raven`. Compilar y testear se podía; desplegar no.

#### Decisiones tomadas

- **Meta:** top 3 del hackathon al 30 de septiembre. No se optimiza para escalar después.
- **Arquitectura de verificación: challenge-response con backend firmante.** El QR lleva
  `quest_id` y un nonce rotativo; la llave nunca sale del servidor; se firma incluyendo la
  address del usuario. Resuelve exposición de llave, replay e intransferibilidad de una vez.
- **Se aplica el fix de front-running**, que es consecuencia de lo anterior.
- **Modelo de trabajo:** el agente escribe, compila, testea e itera. El usuario revisa.
  Reemplaza la regla previa de "no compilo yo".
- **Rama:** se trabaja directo sobre `master`.
- La regla de reutilización del hackathon quedó **descartada como riesgo** por el usuario: lo
  que se reusa es el concepto, y el valor está en llevarlo a cabo.

#### Estado

F0 pendiente: escribir `quest-manager/src/test.rs` y dejar el workspace compilando entero.
