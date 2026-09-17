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
4. **Una app Android de verdad**, no una web guardada en el escritorio: APK
   hecho con Expo / React Native, camara nativa y llave secreta en almacenamiento
   seguro del dispositivo. `scan` y `wallet` contra cadena; `index`, `quest` y
   `profile` se portan como estan, sin funcionalidad nueva.
5. **Repo autoexplicativo.** README que un juez sigue para desplegar y correr.

6. **La presencia tiene que ser real.** El QR lo muestra una pantalla en el
   local y rota cada 30 segundos. Un screenshot de ayer no sirve.

**Fuera de alcance, declarado:** dinero real, passkeys, mapa real, comercios
reales, off-ramp a CLP, **iOS** (compilar para iPhone necesita una Mac). El
**relay** queda abierto y se documenta: quien esta en el lugar puede pasarle el QR
a un tercero por videollamada dentro de la ventana de 30 segundos. Cerrarlo pide
proximidad real (NFC o BLE).

## Estado

F0, F1 y F2 estan hechas. Los puntos 1, 2 y 3 de arriba **ya ocurren contra
testnet**: `node scripts/smoke-testnet.mjs` los corre de punta a punta y sale
`TODO OK`. Lo que falta es que eso pase **desde una app Android** en vez de desde
un script.

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

### F3 - App movil y presencia real — **backend a medias, app por empezar**

Hecho (16/09), sin tocar ninguna pantalla:
- Adapter `@astrojs/node`. Las paginas siguen prerenderizadas; solo `/api` corre
  en el servidor.
- `POST /api/location/challenge` emite un nonce de un solo uso, 5 min de vida,
  atado a su quest.
- `POST /api/location/sign` valida el nonce y firma `quest_id || nonce || address`.
  La address la codifica el servidor.
- Llaves ed25519 por quest desde `.env`. **Nunca en el repo.**
- `scripts/smoke-signer.mjs` levanta el build, hace el handshake y acuna con esa
  firma contra testnet.

**Hueco encontrado el 17/09:** `/challenge` recibe solo `quest_id`, y los quest_id
son 1, 2 y 3. Cualquiera pide un nonce desde su casa, pide la firma con su address
y acuna sin haber ido nunca. El QR no aportaba ningun secreto. No rompe el
anti-replay ni la intransferibilidad; rompe la presencia, que es el producto.

Falta, en este orden:

1. **Firmante rotativo.** Se invierte quien pide el nonce: una pantalla en el local
   (`/local/[quest_id]`) llama a `/challenge` cada 30s y dibuja el QR; el telefono
   solo conoce ese nonce escaneandolo. `/challenge` deja de ser publico. Smoke test
   que verifica que un nonce viejo se rechaza.
2. **App Expo.** Proyecto React Native, las 5 pantallas portadas del diseno actual,
   `expo-camera` para el QR y `expo-secure-store` para la llave. El
   `@stellar/stellar-sdk` 17 corre en RN con un solo polyfill,
   `react-native-get-random-values` — verificado leyendo sus dependencias:
   `@noble/ed25519`, `@noble/hashes` y `@stellar/js-xdr` (cero deps, sin `Buffer`).
3. **`scan` y `wallet` contra cadena.** Escanear -> `/sign` -> `complete_quest`, y
   balance real leido del contrato.
4. **Hosting del firmante.** HTTPS y vivo el dia de la demo; sin esto el telefono
   no llega a ningun lado. La base URL va configurable en la app.
5. **Build del APK.** Por EAS en la nube (pide cuenta de Expo) o instalando el
   Android SDK local. JDK 21 ya esta en la maquina; SDK y `adb` no.

**Listo cuando:** el recorrido completo funciona desde el APK en un telefono real,
contra el firmante hosteado, no desde un script.

### F4 - Demo y entrega
- Guion de demo, con los tres ataques como climax.
- Video. README para el juez. Submission.

**Listo cuando:** entregado antes del 30 a las 22:00 UTC.

## Riesgos

| Riesgo | Mitigacion |
|---|---|
| El arbol de auth de `redeem` falla desde el browser | Ya funciona con la cuenta del usuario como source; replicar eso en la app |
| Camara/QR en movil | `expo-camera` es nativo; fallback de codigo escrito a mano |
| Build del APK sin Android SDK local | EAS en la nube; si falla, se instala el SDK |
| El firmante hosteado se cae el dia de la demo | Tunel local como plan B, y el video grabado antes |
| Contratos archivados antes de la demo | TTL de ~30 dias y se extiende en cada escritura |
| Una llave de ubicacion se filtra | Es demo y estan en `.env`; regenerar con `new-location-keys.mjs` y re-registrar |

## Preguntas abiertas

1. **Build del APK:** cuenta de Expo para EAS en la nube, o instalo el Android SDK
   local? JDK 21 ya esta; falta SDK y `adb`.
2. **Donde se hostea el firmante?** Necesita HTTPS y estar vivo el dia de la demo.
   Hay dominio o cuenta en algun lado, o elijo yo?
3. **Pantalla del local:** hay un segundo dispositivo (tablet, telefono viejo) o se
   usa el laptop?
4. **Las 3 quests de Santiago:** el usuario las decide despues; se sigue con
   placeholders. Solo afecta nombre, foto y coordenadas - la cadena ya tiene las 3
   registradas (5, 3 y 8 WQ).
5. **Wallet del usuario:** la recomendacion es keypair generado por la app y
   guardado en `expo-secure-store`, sin registro ni email. Falta confirmar si se
   agrega ademas un "pegar mi secreta de testnet" para quien ya tenga cuenta.
6. **Las bases del hackathon:** no se pudieron leer, `demo.stellarpassport.xyz` es
   un SPA que trae el contenido por JS. Pendiente que el usuario las pegue.
7. **Arquitectura y patrones de diseno de la app, decididos ANTES de escribir.**
   El usuario pide explicitamente buena escritura y buena arquitectura, no codigo
   que crece solo. Hay que cerrar: estructura de carpetas, donde vive el estado,
   como se separa la capa de cadena de la UI, que patrones se usan y cuales se
   descartan. Se define al empezar la app Expo, antes del primer componente.
8. **Usar Claude Design primero para alguna parte?** El usuario quiere evaluar
   apoyarse en eso antes de portar las pantallas a React Native. Falta definir para
   que exactamente: las 5 pantallas, solo las nuevas, o el sistema de diseno.
