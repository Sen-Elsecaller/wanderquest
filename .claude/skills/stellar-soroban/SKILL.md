---
name: stellar-soroban
description: Escribir, compilar y testear contratos Soroban de WanderQuest con soroban-sdk 27. Usar SIEMPRE antes de tocar cualquier cosa en contracts/, antes de responder sobre APIs de soroban-sdk, stellar-cli, el JS SDK o cualquier SEP, y antes de cablear el frontend a la cadena. Cubre donde verificar cada API sin inventar, que cambio en 27.x, y las restricciones de red del entorno.
---

# Soroban en WanderQuest

## Regla de oro

**Nunca escribir una firma de API de memoria.** El SDK se mueve rapido y una firma
inventada cuesta un ciclo de build completo. Antes de escribir, verificar contra una
de las dos fuentes locales de abajo. Si ninguna responde, decirlo en vez de suponer.

## Las dos fuentes de verdad, en orden

**1. El codigo fuente del SDK, vendorizado en disco.** Es lo mas confiable: es
literalmente lo que compila.

```bash
SDK=$(ls -d /root/.cargo/registry/src/*/soroban-sdk-27.0.6)
grep -n "pub fn " $SDK/src/address.rs
sed -n '/pub trait ToXdr/,/^}/p' $SDK/src/xdr.rs
```

Si no existe esa ruta (maquina distinta, registry limpio), se repuebla con
`cargo fetch` dentro de `contracts/`.

**2. La documentacion oficial en markdown.**

```bash
ls /home/user/stellar/stellar-docs/docs/tokens/token-interface.mdx
```

Si no esta clonada:
`GIT_LFS_SKIP_SMUDGE=1 git clone --depth 1 https://github.com/stellar/stellar-docs /home/user/stellar/stellar-docs`

**3. MCP `stellar-raven`** (`raven.stellar.buzz/mcp`) - gateway oficial-experimental
a doc y datos vivos, pensado para agentes. **Bloqueado desde este contenedor** (403).
Sirve solo en el PC del usuario.

## Verificado en soroban-sdk 27.0.6

Todo lo de esta seccion esta comprobado contra el fuente o la doc, no de memoria.

### Los eventos cambiaron. `publish` esta deprecado.

`env.events().publish(...)` emite warning de deprecacion. La API actual es el macro
`#[contractevent]` (`soroban-sdk/src/lib.rs:1056`).

### Meter una `Address` en un mensaje firmado: usar `to_xdr`

`ToXdr::to_xdr(self, env) -> Bytes` (`src/xdr.rs:48`) tiene impl blanket para todo
`T: IntoVal<Env, Val>`, asi que aplica a `Address`, y produce bytes deterministas.

```rust
let mut message = Bytes::new(&env);
message.extend_from_array(&quest_id.to_be_bytes());
message.extend_from_array(&nonce.to_array());
message.append(&user.to_xdr(&env));
```

**NO usar `Address::to_payload()`** aunque parezca mas directo: esta tras
`#[cfg(feature = "hazmat-address")]` y su propia doc desaconseja explicitamente
usarlo para verificacion de firmas, porque la master key de una cuenta puede no
ser signer de esa cuenta.

El firmante off-chain tiene que producir exactamente los mismos bytes: XDR del
`ScVal::Address`. Con el JS SDK sale de `Address.toScVal().toXDR()`.

### SEP-41 exige mas de lo que parece

Segun `docs/tokens/token-interface.mdx`, la interfaz completa incluye `allowance`,
`approve`, `transfer_from` y `burn_from`, ademas de lo obvio. Un token sin eso no
es interoperable y los explorers no lo muestran bien.

**En 27.0.6 el `to` de `transfer` es `MuxedAddress`, no `Address`.**

### Storage y TTL

Los balances viven en `persistent` con bump de TTL. El `instance` storage (admin,
supply, owner, direccion del token) **tambien expira** y es facil olvidarlo: si el
contrato se archiva, deja de responder. Extenderlo tambien.

## Patrones de este proyecto

- **`QuestManager` es el unico `admin` de `WQToken`.** Es la propiedad central del
  sistema: WQ nuevo solo nace de una visita verificada. No agregar otra via de mint.
- **La prueba de visita se ata al usuario.** El mensaje firmado es
  `quest_id || nonce || address_del_usuario`. Sin la address, la prueba es al portador
  y se puede regalar o robar.
- **La llave ed25519 de cada ubicacion vive en el backend firmante**, nunca en el QR
  ni en un tag pasivo ni en el repo. Quien tiene la llave puede acuñar sin moverse.
- **Un contrato llamando a otro se autoriza solo.** `QuestManager` invoca
  `token.mint(...)` y el token hace `admin.require_auth()`: se satisface porque el
  invocador es el propio admin.
- **`redeem` necesita que la auth del usuario cubra tres nodos** (`redeem`,
  `transfer`, `burn`). Testearlo antes de cablear el frontend.

## Comandos

```bash
cd contracts
cargo test                                        # tests, entorno local sin red
cargo build --target wasm32v1-none --release      # wasm
cargo clippy --all-targets
```

`cargo test` corre en un host Soroban local: **no necesita red**, funciona aca.

## Lo que este contenedor NO puede hacer

`horizon-testnet.stellar.org`, `soroban-testnet.stellar.org` y
`friendbot.stellar.org` dan **403** en el proxy de egress. O sea: **no se puede
desplegar, ni fondear cuentas, ni consultar la red desde aca.**

Escribir, compilar, testear y generar el wasm: si. Desplegar: lo corre el usuario
en su maquina, o se habilitan esos dominios en la politica de red del entorno.

No intentar rodear esto desactivando TLS ni tocando `HTTPS_PROXY`.

## Seguridad

- Las llaves secretas las custodia el usuario. La llave de firma de ubicacion es
  material sensible aunque sea demo.
- Nunca commitear una llave, ni pegarla en un log, ni mandarla a ningun servicio.
- Todo en **testnet**. Nunca mainnet ni fondos reales sin pedido explicito.
