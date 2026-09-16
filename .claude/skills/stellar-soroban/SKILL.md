---
name: stellar-soroban
description: Escribir, compilar, testear y desplegar los contratos Soroban de WanderQuest con soroban-sdk 27 y stellar-cli 28. Usar SIEMPRE antes de tocar cualquier cosa en contracts/, antes de responder sobre APIs de soroban-sdk, stellar-cli, el JS SDK o cualquier SEP, y antes de cablear el frontend a la cadena. Cubre donde verificar cada API sin inventar, que cambio en 27.x, las trampas ya encontradas y como se despliega.
---

# Soroban en WanderQuest

## Regla de oro

**Nunca escribir una firma de API de memoria.** El SDK se mueve rapido y una firma
inventada cuesta un ciclo de build completo. Antes de escribir, verificar contra una
de las fuentes locales de abajo. Si ninguna responde, decirlo en vez de suponer.

## Las fuentes de verdad, en orden

**1. El codigo fuente del SDK, vendorizado en disco.** Es lo mas confiable: es
literalmente lo que compila.

```bash
SDK=$(ls -d /c/Users/Sen/.cargo/registry/src/*/soroban-sdk-27.0.6)
grep -n "pub fn " $SDK/src/address.rs
sed -n '/pub trait ToXdr/,/^}/p' $SDK/src/xdr.rs
```

El crate de macros esta al lado (`soroban-sdk-macros-27.0.6`) y contesta lo que el
SDK no dice: `src/derive_event.rs` fija como se arman los topics y el data de un
`#[contractevent]`, `src/lib.rs` como `contractimpl` trata un `impl Trait for`.

Si no existe esa ruta (registry limpio), se repuebla con `cargo fetch` dentro de
`contracts/`.

**2. La documentacion oficial en markdown**, clonada en
`C:\Users\Sen\stellar\stellar-docs` (en bash: `/c/Users/Sen/stellar/stellar-docs`).

```bash
D=/c/Users/Sen/stellar/stellar-docs
cat $D/docs/tokens/token-interface.mdx                                  # SEP-41
cat $D/docs/learn/fundamentals/contract-development/storage/state-archival.mdx
```

Si no esta clonada:
`GIT_LFS_SKIP_SMUDGE=1 git clone --depth 1 https://github.com/stellar/stellar-docs ~/stellar/stellar-docs`

**3. La red misma.** `stellar contract invoke ... --send=no` lee estado real sin
firmar nada. Para dudas de comportamiento, preguntarle a testnet sale mas barato
que discutirlo.

**4. MCP `stellar-raven`** (`raven.stellar.buzz/mcp`) - gateway oficial-experimental
a doc y datos vivos. El host responde desde este PC; el MCP no esta conectado a la
sesion. Si se conecta, es una fuente mas, nunca por encima del fuente del SDK.

## Entorno

Todo corre en el PC del usuario (Windows). El contenedor sin red quedo atras:

| | Estado |
|---|---|
| Rust | 1.98.1, host `x86_64-pc-windows-gnu`, target `wasm32v1-none` |
| stellar-cli | 28.0.0, en `C:\Program Files (x86)\Stellar CLI` |
| Testnet | **alcanzable** - horizon, RPC y friendbot responden |
| Contratos | **desplegados**, ver `deployments/testnet.json` |

**No hay MSVC Build Tools en la maquina.** El host es el toolchain GNU, y su linker
desborda su limite de 65535 ordinales al exportar el `cdylib` de un contrato. Por
eso existe `contracts/.cargo/config.toml` con
`-C link-arg=-Wl,--exclude-all-symbols`: sin eso `cargo test` no linkea. Afecta solo
al build de host; el wasm usa `rust-lld` y no pasa por ahi.

## Comandos

```bash
cd contracts
cargo test                                        # host, sin red
cargo build --target wasm32v1-none --release      # wasm
cargo clippy --all-targets
cargo fmt --all
```

```powershell
pwsh scripts/deploy-testnet.ps1     # despliega, cablea y registra quests
node scripts/smoke-testnet.mjs      # recorrido completo + los 3 ataques, en vivo
```

Los exports reales de un wasm, que es lo unico que prueba que el contrato quedo
bien armado:

```bash
node -e "const fs=require('fs');console.log(WebAssembly.Module.exports(new WebAssembly.Module(fs.readFileSync('contracts/target/wasm32v1-none/release/quest_manager.wasm'))).map(e=>e.name).sort().join(', '))"
```

## Trampas ya pagadas

### Depender del crate de otro contrato mete sus exports en tu wasm

`quest-manager` tenia `wq-token = { path = "../wq-token" }` en `[dependencies]` solo
para usar `WQTokenClient`. Eso linkea los `#[contractimpl]` del token dentro del wasm
del manager: `quest_manager.wasm` exportaba `mint`, `burn` y `transfer`, y los dos
`initialize` (de distinta aridad) colisionaban, asi que el linker se comia el del
manager. El contrato no se podia ni inicializar.

La forma correcta de llamar a otro contrato es declarar solo la interfaz:

```rust
#[contractclient(name = "TokenClient")]
pub trait TokenInterface {
    fn mint(env: Env, to: Address, amount: i128);
}
```

El crate del otro contrato puede quedar como `[dev-dependencies]` para los tests.
**Despues de tocar dependencias entre contratos, mirar los exports del wasm.**

### Los eventos cambiaron. `publish` esta deprecado

`env.events().publish(...)` emite warning de deprecacion. La API actual es el macro
`#[contractevent]` (`soroban-sdk/src/lib.rs:1056`), y el evento se emite con
`MiEvento { .. }.publish(&env)`.

- El topic por defecto es el nombre del struct en snake_case; `topics = [...]` lo
  cambia; cada campo `#[topic]` se agrega despues.
- `data_format`: mapa (por defecto), `"vec"` o `"single-value"` (un solo campo).

### En los tests, `env.events().all()` solo trae la ultima invocacion

Leer los eventos **antes** de cualquier otra llamada al contrato, porque cada
`client.balance(...)` es una invocacion y vacia el buffer.

### Meter una `Address` en un mensaje firmado: `to_xdr`

`ToXdr::to_xdr(self, env) -> Bytes` (`src/xdr.rs:48`) tiene impl blanket para todo
`T: IntoVal<Env, Val>`, asi que aplica a `Address` y produce bytes deterministas.

```rust
let mut message = Bytes::new(&env);
message.extend_from_array(&quest_id.to_be_bytes());
message.extend_from_array(&nonce.to_array());
message.append(&user.clone().to_xdr(&env));
```

Del lado JS, los mismos bytes salen de `Address.fromString(g).toScVal().toXDR()`
(verificado en vivo contra el contrato desplegado).

**NO usar `Address::to_payload()`** aunque parezca mas directo: esta tras
`#[cfg(feature = "hazmat-address")]` y su propia doc desaconseja explicitamente
usarlo para verificacion de firmas.

Para firmar en Rust desde los tests, `Bytes::to_buffer::<N>()` da un `&[u8]` sin
necesitar `alloc`.

### SEP-41: implementar el trait del SDK, no copiarlo

`soroban_sdk::token::TokenInterface` es un `#[contracttrait]`. Implementarlo con
`#[contractimpl] impl token::TokenInterface for WQToken` hace que las firmas las
garantice el SDK. Lo que no es parte de la interfaz (`initialize`, `mint`, `admin`,
`total_supply`) va en un bloque `#[contractimpl] impl WQToken` aparte; los dos
bloques alimentan el mismo `WQTokenClient`.

**El `to` de `transfer` es `MuxedAddress`, no `Address`** (27.0.6). `to.address()`
da la cuenta subyacente y `to.id()` el id de multiplexacion, que va en el evento.
Un `Address` plano sirve como argumento en el cable, asi que un contrato que llame
`transfer` puede declarar `to: Address` en su cliente.

Formas de evento (`docs/tokens/token-interface.mdx`): `["transfer", from, to]` con
data `{ amount, to_muxed_id }`, `["burn", from]` y `["mint", to]` con data `amount`,
`["approve", from, spender]` con data `[amount, live_until_ledger]`.

### Storage y TTL

- `instance().extend_ttl()` extiende la instancia **y el codigo** del contrato. Sin
  eso el contrato se archiva y deja de responder.
- `persistent` y `temporary` se extienden entrada por entrada.
- Una entrada `persistent` archivada **no se puede recrear, solo restaurar**
  (`state-archival.mdx`), asi que el registro de nonces usados sobrevive a su TTL.
- `temporary` archivada **se borra**: sirve para allowances, que ya tienen su propio
  vencimiento.

## Patrones de este proyecto

- **`QuestManager` es el unico `admin` de `WQToken`.** WQ nuevo solo nace de una
  visita verificada. No agregar otra via de mint.
- **La prueba de visita se ata al usuario.** El mensaje firmado es
  `quest_id || nonce || address_del_usuario`. Sin la address, la prueba es al
  portador y se puede regalar o robar.
- **La llave ed25519 de cada ubicacion vive en el backend firmante**, nunca en el QR
  ni en un tag pasivo ni en el repo. En `.env` (gitignoreado), una por quest.
- **Un contrato llamando a otro se autoriza solo.** `QuestManager` invoca
  `token.mint(...)` y el token hace `admin.require_auth()`: se satisface porque el
  invocador es el propio admin.
- **`redeem` necesita que la auth del usuario cubra tres nodos** (`redeem`,
  `transfer`, `burn`). Testeado con `mock_auths`, y en vivo sale solo cuando el
  usuario es la cuenta que firma la transaccion.

## Seguridad

- Las claves secretas las custodia el usuario. La llave de firma de ubicacion es
  material sensible aunque sea demo: no se imprime, no se commitea, no se manda a
  ningun servicio.
- La identidad de deploy es `wanderquest-admin` en el keystore de stellar-cli.
- Todo en **testnet**. Nunca mainnet ni fondos reales sin pedido explicito.
