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
`#signer` `#frontend` `#mobile` `#tooling` `#docs` `#process-rules`

**La fecha es la del día, y punto.** No se deduce de la última entrada ni se incrementa. Si
ya hay una entrada de hoy, la nueva es `(Sesión 2)`, `(Sesión 3)` en el título. La semana se
titula con su rango de fechas, no con un número de semana del proyecto: hubo seis meses entre
el commit inicial y el trabajo real, y numerar desde marzo mentiría sobre el ritmo.

**Si la fecha de hoy contradice lo que dice la última entrada, la equivocada es la entrada:**
corregirla, no acomodar la nueva alrededor. `git log` la verifica.

---

## Semana 14 - 20 de septiembre

### Sesión: 2026-09-17 — La app se vuelve nativa, y el QR deja de ser decorativo

**Tags:** #mobile #frontend #security #signer #tooling #docs

Sesión sin una línea de código: se fue entera en dos preguntas del usuario que resultaron ser
las dos decisiones más grandes que quedaban. La primera —*"entiendo que sería nativa de
Android, no desde el navegador"*— destapó que nadie había decidido eso nunca. La segunda
—*"¿qué tal React Expo para esto?"*— dio vuelta la recomendación que yo mismo acababa de dar.

#### El hueco que estaba a la vista y nadie miró

Revisando qué tendría que hacer `scan`, salió que el `CLAUDE.md` decía una cosa y el código
hacía otra. El doc: *"el QR lleva `quest_id` y un nonce rotativo"*. `challenge.ts`: recibe
**sólo `quest_id`** y emite el nonce él mismo.

Los quest_id son 1, 2 y 3. O sea que desde cualquier lado se pide un nonce, se pide la firma
con la address propia, y se acuña. **Nunca hay que ir al Cerro Santa Lucía.** El anti-replay y
la intransferibilidad funcionan perfecto; lo que no estaba atado a nada era la presencia
física, que es literalmente el producto. El CPVV entero descansaba sobre un QR que no aportaba
ningún secreto.

El arreglo no necesita contratos nuevos ni firmante nuevo: **se invierte quién pide el
nonce.** Una pantalla en el local (`/local/[quest_id]`) llama a `/challenge` cada 30 segundos
y dibuja el QR; el teléfono sólo puede conocer ese nonce escaneándolo. `/challenge` deja de
ser público. El nonce vive 30 segundos, existe únicamente en una pantalla que está físicamente
en el lugar, y se gasta al primer uso.

Dos consecuencias que se escriben, no se esconden:

- **Un QR rotativo no se puede imprimir.** El comercio necesita una pantalla —una tablet
  vieja, un teléfono—. En la demo eso juega a favor (el QR cambiando en cámara es el
  argumento), pero en el modelo real es un costo por local.
- **El relay sigue abierto.** Alguien parado en el cerro le pasa el QR por videollamada a un
  amigo y el amigo acuña. Cerrarlo pide proximidad real, NFC o BLE, y queda fuera de alcance.
  Pero el ataque pasó de *"cualquiera, desde su casa, para siempre"* a *"necesitás un cómplice
  presente y coordinado en una ventana de 30 segundos"*. Esa diferencia es el producto.

#### Nativa: primero Capacitor, después Expo, y por qué cambió

El estado real era que la app no existía: las cinco pantallas son Astro + Tailwind, y
`scan.astro` tenía una **cámara simulada** —un div con un marco y texto, sin `getUserMedia`—.

La primera recomendación fue **Capacitor**: envolver el frontend que ya existe en un APK,
ganando ML Kit para el QR y Android Keystore para la llave secreta —que de paso arregla solo
el punto flojo de la wallet, porque `localStorage` se lo lleva cualquier XSS—. Kotlin nativo
quedó descartado por una razón que no es el tiempo: obligaría a migrar a `java-stellar-sdk` y
re-verificar todo el armado de transacciones Soroban, que es justo la parte delicada y ya
probada contra la red.

Entonces el usuario preguntó por Expo, y **la suposición que iba a costar la decisión era
mía**: di por hecho que `@stellar/stellar-sdk` no sobreviviría a React Native, porque esas
libs históricamente arrastran `Buffer`, `crypto` y `sodium-native`. Mirar el
`package.json` en vez de confiar en la memoria dijo otra cosa:

| Dependencia de stellar-sdk 17.1.0 | Qué es |
|---|---|
| `@noble/ed25519`, `@noble/hashes` | JS puro, sin crypto nativo |
| `@stellar/js-xdr` 5.0.0 | **cero dependencias**, ni una referencia a `Buffer` en `lib/` |
| `@exodus/bytes`, `uint8array-extras` | todo sobre `Uint8Array` |

La capa de cadena corre en RN con **un solo polyfill**, `react-native-get-random-values`. El
costo de Expo no es la cadena: es el frontend, porque RN no tiene DOM y las cinco pantallas se
reescriben.

El usuario eligió **Expo**, y el motivo que dio ordena el resto del proyecto: *"quiero hacer la
prueba de hasta qué punto te puedo encargar una app móvil"*, con el tiempo explícitamente
fuera de la ecuación. Con Capacitor la app parece nativa; con Expo lo es. iOS queda fuera:
compilar para iPhone necesita una Mac.

#### Lo que el entorno ya tiene y lo que falta

Node 24.14.0, npm 11.9.0 y **JDK 21.0.11 ya instalado**. No hay Android SDK ni `adb`, así que
el build sale por **EAS en la nube** —que pide una cuenta de Expo del usuario— o instalando el
SDK local. Es la primera pregunta de la próxima sesión.

Las bases del hackathon no se pudieron leer: `demo.stellarpassport.xyz` es un SPA de Next.js
que trae el contenido por JS, y el HTML servido es sólo el shell. Quedó pendiente que el
usuario las pegue.

#### Dos encargos que el usuario dejó anotados

Los dos son para resolver al empezar la app, no ahora: **decidir la arquitectura y los
patrones de diseño antes de escribir la primera línea** —el pedido textual fue *"quiero una
buena escritura y arquitectura de código"*, o sea que la estructura se piensa, no se descubre
mientras crece—, y **evaluar apoyarse en Claude Design** para alguna parte del port de las
pantallas. Quedaron como preguntas 7 y 8 de [plan.md](plan.md).

#### Estado

Nada de código escrito, nada commiteado salvo estos docs. La cadena sigue igual que ayer: dos
contratos desplegados, 22 tests verdes, los dos smoke tests en `TODO OK`. Lo que cambió es el
plan: **F3 se redefine** —firmante rotativo, app Expo, hosting— y el alcance pasa de "cablear
dos pantallas web" a "una app Android de verdad". Las tres quests de Santiago siguen sin
decidir por elección del usuario; se sigue con placeholders.

---

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
