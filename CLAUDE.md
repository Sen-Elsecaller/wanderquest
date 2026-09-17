# WanderQuest

Plataforma de exploración urbana gamificada sobre **Stellar**. Completás quests físicas
(QR/geo), ganás tokens **WQ** canjeables en comercios locales. El modelo es **CPVV** (cost
per verified visit): el comercio paga por visitas reales verificadas, no por métricas
infladas.

**Qué existe hoy:** landing page + prototipo de app móvil navegable (5 pantallas), puro
frontend. **Qué estamos construyendo:** la capa on-chain real — que WQ sea un token de
verdad y que "visita verificada -> acuñar -> canjear" liquide en la red.

**Objetivo inmediato:** submission para el hackathon **"Find Your Way"** (Stellar Passport),
General Track. Deadline de envío **2026-09-30 22:00 UTC**. Todo en **testnet**.

---

## Estado del proyecto (temporal — actualizar al avanzar)

Marca el estado real de cada pieza: `escrito` / `compila` / `testeado` / `deployado`.

- **`contracts/wq-token`** — token WQ, SEP-41 completo vía `token::TokenInterface` del SDK, más
  `mint` admin-gated. Estado: **desplegado en testnet, 11 tests pasan**.
- **`contracts/quest-manager`** — verifica firma ed25519 de la visita on-chain y acuña; maneja
  canje con fee/quema. Estado: **desplegado en testnet, 11 tests pasan**.
- **Deploy** — `deployments/testnet.json` tiene los IDs. `scripts/deploy-testnet.ps1` rehace todo
  de cero; `scripts/smoke-testnet.mjs` corre el recorrido completo y los 3 ataques contra la red.
- **Backend firmante** — `POST /api/location/challenge` y `POST /api/location/sign` (Astro con
  adapter node; solo `/api` es SSR, las páginas siguen estáticas). Estado: **funciona contra
  testnet** — `npm run smoke:signer`. Las llaves ed25519 de las 3 ubicaciones viven en `.env`
  (gitignoreado); sus públicas están registradas on-chain.
- **Frontend** — landing + 5 pantallas mock. Estado: **hecho, sin cablear a cadena**. Es lo único
  que falta de F3 y lo primero que toca diseño.
- **Toolchain** — todo corre en el PC del usuario (Windows): Rust 1.98.1 host `x86_64-pc-windows-gnu`,
  target `wasm32v1-none`, `stellar-cli` 28.0.0, Node 24. No hay MSVC Build Tools; ver la nota del
  linker en la skill. MCP `stellar-raven` no está conectado a la sesión (el host sí responde).
- **Red** — testnet **alcanzable**. Compilar, testear, desplegar y leer estado: todo desde acá.

**Decisiones cerradas** (no reabrir sin motivo nuevo): WQ = contrato Soroban (no asset
clásico); wallet = keypair en la app (demo); alcance = profundidad sobre el flujo core;
verificación de visita = firma ed25519 verificada on-chain; off-ramp a CLP = simulado;
**la llave ed25519 de cada ubicación vive en un backend firmante** (nunca en el QR ni en un
tag pasivo); **la prueba de visita se ata al usuario** — se firma `quest_id || nonce ||
address` (implementado y probado en vivo); **Rust queda en 4 espacios con rustfmt por
defecto** (la preferencia de tabs del usuario aplica al resto); meta = **top 3 al
2026-09-30**, no escalar antes de eso.

**Abierto — decisiones del usuario:**

1. **Las 3 quests de Santiago.** Cerro Santa Lucía ya tiene imagen en `public/`; faltan dos.
   No bloquea la cadena: el contrato solo guarda llave pública, recompensa y estado — el
   nombre, la foto y las coordenadas viven en el frontend. Las quests 1, 2 y 3 ya están
   registradas en testnet con 5, 3 y 8 WQ.

El resto de preguntas abiertas, al final de `docs/plan.md`.

---

## Reglas de Interacción

- Conciso. Sacrificar gramática por concisión.
- **Nada de `→` (flecha unicode) en código, comentarios, strings ni docs** — usar `->` ASCII.
  Evita problemas de encoding.
- **Verificar antes de escribir Soroban/Stellar, nunca de memoria.** Ante cualquier duda de API
  (soroban-sdk, stellar-cli, JS SDK, SEP), leer la skill `.claude/skills/stellar-soroban/`:
  dice dónde está el fuente del SDK vendorizado y la doc oficial clonada, ambos en disco.
  El SDK evoluciona rápido; una firma inventada cuesta un ciclo de build.
- **Yo cargo el peso.** Escribo todo el código: contratos, frontend, configs, tests, bindings,
  docs. **Compilo, corro los tests, despliego a testnet y verifico contra la red hasta que pase
  todo** — no le paso errores de compilación al usuario. Él decide y revisa.
- **Mainnet y fondos reales siguen siendo del usuario.** Testnet la manejo yo; cualquier cosa
  con valor real se pregunta antes.
- **Commit y push a `master`** cuando el usuario lo pide o cuando el hook de cierre lo exija;
  avisando siempre qué se commiteó y por qué.
- **No documentar una implementación como funcional hasta que compile y se pruebe.** Se escribe
  el estado real (ver arriba). Un doc sobre una suposición se lee después como un hecho.
- **Preguntar antes de asumir.** Si una instrucción, tarea o consulta no queda clara, preguntar.
- **Fin de cada plan: listar las preguntas abiertas, muy conciso.**
- **Ante una idea de implementación a medias**, 3 pasos en orden: (1) analizar mejoras, huecos,
  casos no cubiertos, alternativas más simples; (2) consultar las dudas necesarias; (3) solo si
  no hay dudas ni mejoras, describir cómo se implementaría sin tocar código. No saltar a proponer
  con información incompleta.
- **No tratar lo ya construido como restricción fija** al analizar algo nuevo — todo lo previo
  puede moldearse si un nuevo paradigma lo pide. Excepción: lo que el usuario marque como fijo.
- **Modo conversación de diseño/ideas** — lo dispara el usuario o el tema (concepto, economía del
  token, UX, gameplay, narrativa). Ahí se resuelve en términos de diseño y valor para el usuario
  final, no de costo. "Ya existe", "reusa X", "es casi gratis" no hacen una idea mejor ni peor.
  Un dato del repo corrige un hecho, no rankea una opción. Al buscar referencias, barrer todo, no
  la carpeta obvia. Se sale del modo al pasar a *cómo* construirlo.
- **Ante un "¿por qué?" explicar genuinamente** — el foco es la explicación técnica real, no
  asumir que es una corrección.
- **Un comentario describe el código como está, no cómo llegó a estar.** Alarma: "antes", "ya no",
  "pasó a ser", o mencionar algo que no está en el archivo. Reescribir en presente.
- **Leer antes de editar** — el archivo/módulo completo (imports incluidos) antes de modificar o
  afirmar que algo no existe.
- **Casos concretos primero.** La infra compartida sale de duplicados reales, no por adelantado.
- **Usar features actuales con confianza** (soroban-sdk 27, stellar-cli 28, Astro 5, Tailwind v4),
  sin disclaimers de "a verificar" una vez confirmada la API.
- **Todo en testnet.** Nunca mainnet ni fondos reales salvo pedido explícito del usuario.
- **Nunca exponer ni enviar claves secretas.** Las claves las custodia el usuario; la clave de
  firma de ubicación (ed25519) es material sensible aunque sea demo.

### Estilo de los `.md`

Documentación de referencia viva, no bitácora. Cuando algo cambia, **editar el texto existente
in-place** y barrer el doc por lo que quedó desactualizado — no acumular secciones por sesión.

**Excepción: `docs/bitacora.md`** es histórico. Va en **orden reverso-cronológico** (lo más
reciente arriba), agrupado por semana, una sección por sesión: título con fecha, tags, y prosa
que cuenta **por qué** se hizo cada cosa, no un listado de qué se tocó. Cierra con `#### Estado`.
Sus propias reglas de formato están en su cabecera — leerlas antes de escribir una entrada.

### `CLAUDE.md` es instrucciones + router, no la enciclopedia

Se carga entero en cada turno. El detalle profundo vive en su propio archivo:

| Dónde | Qué |
|---|---|
| `docs/plan.md` | Plan al 2026-09-30, fases, riesgos, preguntas abiertas |
| `docs/bitacora.md` | Histórico de sesiones (más reciente arriba): decisiones, hallazgos, por qué |
| `.claude/skills/stellar-soroban/` | Cómo verificar APIs de Soroban y qué cambió en 27.x |

---

## Estructura del Proyecto

```
wanderquest/
├── src/                      # Astro: landing + app (frontend, ya existe)
│   ├── components/landing/   # Secciones de la landing
│   ├── layouts/              # Layout (web) y AppLayout (app móvil)
│   ├── pages/                # / (landing) y /app/* (5 pantallas)
│   ├── pages/api/location/   # challenge + sign (SSR; el resto es estático)
│   ├── lib/                  # location-signer (server only: llaves ed25519)
│   └── styles/global.css     # Tailwind v4 (@theme) + estilos app
├── contracts/                # Workspace Rust/Soroban
│   ├── Cargo.toml            # workspace + perfil release para Wasm
│   ├── Cargo.lock            # fija soroban-sdk 27.0.6 — commiteado a propósito
│   ├── .cargo/config.toml    # flag del linker mingw (ver skill)
│   ├── wq-token/             # token WQ
│   └── quest-manager/        # verificación de visita + acuñación + canje
├── scripts/                  # deploy, llaves de ubicación, smoke tests en vivo
├── deployments/testnet.json  # IDs desplegados — público, se commitea
├── .env                      # llaves ed25519 de ubicación — NUNCA se commitea
├── docs/                     # plan y bitácora
├── .claude/skills/           # skills del proyecto
├── public/                   # assets (logos, imágenes, HTMLs de referencia)
└── package.json
```

## Stack y Versiones

| Capa | Tecnología |
|------|------------|
| Frontend | Astro 5 · Tailwind CSS v4 (`@theme`) · GSAP/ScrollTrigger · Vanta.js + Three |
| Contratos | Rust 1.98.1 · soroban-sdk **27.0.6** (fijado en `Cargo.lock`) · target `wasm32v1-none` |
| Cadena (JS) | `@stellar/stellar-sdk` 17 · Node 24 |
| Tooling | `stellar-cli` **28.0.0** · doc oficial clonada en `C:\Users\Sen\stellar\stellar-docs` |
| Red | Stellar **testnet**, alcanzable desde el PC — contratos ya desplegados |

## Arquitectura on-chain

- **`WQToken`** — fungible, 7 decimales, SEP-41 completo (implementa `token::TokenInterface` del
  SDK: allowances incluidas, y el `to` de `transfer` es `MuxedAddress`). Balances en storage
  persistente con bump de TTL. Solo el `admin` puede acuñar; el admin es el `QuestManager`, así
  que WQ nuevo solo nace de una visita verificada.
- **`QuestManager`** — el CPVV. Cada quest tiene la **clave pública ed25519** de su ubicación. Al
  completar, el usuario envía una firma; el contrato la **verifica on-chain**
  (`env.crypto().ed25519_verify`) antes de acuñar la recompensa. El `nonce` usado queda
  marcado (anti-replay). `redeem` mueve WQ del usuario al comercio y quema el 5% (fee de
  recirculación del modelo económico).
- **Mensaje firmado:** `quest_id` (big-endian) `|| nonce || user.to_xdr(env)`. La address dentro
  del mensaje es lo que impide que la prueba sea al portador: una firma filtrada o reenviada no
  le sirve a nadie más. Del lado JS los mismos bytes salen de
  `Address.fromString(g).toScVal().toXDR()`.
- **Verificación de visita: challenge-response.** El QR lleva `quest_id` y un nonce rotativo,
  nunca la llave. Un backend firmante custodia la llave ed25519 de cada ubicación, valida el
  nonce y firma incluyendo la address de quien reclama. Si la llave estuviera en el QR o en un
  tag pasivo, quien lo fotografía acuña desde su casa para siempre y el CPVV se cae.
- **Limitación honesta que queda:** el GPS es falsificable. Lo que el sistema prueba es posesión
  de una firma emitida por el firmante del local; la calidad de la prueba depende de cuán
  estricto sea ese firmante. Se documenta, no se esconde.

## Modelo Económico (referencia)

- WQ **fungible**, canjeable en cualquier comercio aliado. **1 WQ ≈ 100 CLP ≈ $0.10 USD**.
- Fees (destino WanderQuest): depósito de comercio 10%, recirculación 5% (se quema), retiro de
  usuario 20%, canje 0% para el usuario.
- Todo número es provisional hasta tener algo sólido y jugable.

## Pantallas de la app (`/app/*`)

`index` (mapa con quests) · `quest` (detalle) · `scan` (scanner QR + éxito) · `wallet` (balance,
retiro) · `profile` (stats, logros). En el MVP se cablean a cadena: **wallet** (balance real),
**scan** (acuñar por visita verificada) y el flujo de **canje**; mapa y perfil quedan como UI.

## Paleta y Fuentes

- Landing: `--color-primary #0891B2` (teal), `--color-accent #F97316` (coral),
  `--color-navy #0B1E4A`, `--color-off-white #F6F7FA`. Fuentes: Montserrat (headings), Inter (body).
- App móvil: fondo `#F5F8F8`, superficie `#FFFFFF`, `--color-gold #FBBF24`. Fuente: Plus Jakarta Sans.
