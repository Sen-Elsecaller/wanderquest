# WanderQuest Astro — Contexto del Proyecto

## Resumen
WanderQuest es una plataforma de exploración urbana gamificada. Los usuarios descubren lugares, completan misiones físicas y ganan tokens canjeables en comercios locales. Construido sobre Stellar blockchain.

**Estado:** Landing page completa + Prototipo de app móvil navegable (5 pantallas).

---

## Stack Técnico

| Tecnología | Uso |
|------------|-----|
| **Astro 5.17** | Framework principal (static output) |
| **Tailwind CSS v4** | Estilos con `@theme` en global.css |
| **GSAP + ScrollTrigger** | Animaciones scroll-driven con pinning (JourneyMap) |
| **Vanta.js + Three.js** | Fondos 3D animados (Hero con GLOBE, Sponsors con FOG) |
| **Material Icons Round** | Iconografía para app móvil |
| **Plus Jakarta Sans** | Tipografía para app móvil |

---

## Estructura del Proyecto

```
wanderquest-astro/
├── public/
│   ├── wanderquest-logo.png       # Logo WanderQuest (128x128)
│   ├── stellar-logo-small.png     # Logo Stellar (48x41, aspect ratio correcto)
│   ├── cerro-santa-lucia-small.png # Imagen quest (400x400)
│   └── reference app/             # HTMLs de referencia originales (no usados)
├── src/
│   ├── components/
│   │   ├── landing/
│   │   │   ├── Navbar.astro        # Nav oculto, aparece al scroll (con logo real)
│   │   │   ├── Hero.astro          # Hero inmersivo con Vanta.js GLOBE + badge Stellar
│   │   │   ├── JourneyMap.astro    # Ruta con dashes animados (5 pasos) + pinning
│   │   │   ├── ForWho.astro        # Sección "Dos mundos" (exploradores/negocios)
│   │   │   ├── Sponsors.astro      # Ecosistema WQ + carrusel comercios (Vanta FOG)
│   │   │   ├── BusinessCTA.astro   # CTA para comercios (formulario contacto)
│   │   │   ├── FinalCTA.astro      # CTA para usuarios (newsletter)
│   │   │   └── Footer.astro        # Footer con links y logos reales
│   │   └── ui/
│   │       └── AnimatedCompass.astro  # Brújula SVG animada
│   ├── layouts/
│   │   ├── Layout.astro            # Layout para landing page
│   │   └── AppLayout.astro         # Layout para app móvil (incluye BottomNav + logo)
│   ├── pages/
│   │   ├── index.astro             # Landing page
│   │   └── app/
│   │       ├── index.astro         # Mapa con quests cercanas
│   │       ├── quest.astro         # Detalle de quest (con imagen real)
│   │       ├── scan.astro          # Scanner QR con estado de éxito
│   │       ├── wallet.astro        # Billetera con balance y retiro
│   │       └── profile.astro       # Perfil con stats y logros
│   └── styles/
│       └── global.css              # Tailwind + variables @theme + estilos app
├── package.json
└── CLAUDE.md                       # Este archivo
```

---

## Rutas de la Aplicación

| Ruta | Descripción |
|------|-------------|
| `/` | Landing page |
| `/app` | Mapa con quests cercanas (home de la app) |
| `/app/quest` | Detalle de una quest |
| `/app/scan` | Scanner QR con overlay de éxito |
| `/app/wallet` | Billetera, balance WQ, retiro a dinero real |
| `/app/profile` | Perfil de usuario, stats, logros |

**Navegación:** El botón "Descarga la app" en el Navbar redirige a `/app`.

---

## Paleta de Colores (Ocean Quest)

```css
/* Landing */
--color-primary: #0891B2;      /* Teal - principal */
--color-accent: #F97316;       /* Coral - acento */
--color-navy: #0B1E4A;         /* Navy - texto heading */
--color-off-white: #F6F7FA;    /* Fondo principal */

/* App móvil - adicionales */
--color-background-light: #F5F8F8;
--color-surface-light: #FFFFFF;
--color-gold: #FBBF24;
--color-neutral-50 a neutral-900;  /* Escala de grises teal */
```

---

## Fuentes

| Contexto | Fuente | Weights |
|----------|--------|---------|
| Landing headings | Montserrat | 700-900 |
| Landing body | Inter | 400-600 |
| App móvil | Plus Jakarta Sans | 400-800 |

---

## Modelo Económico (Pool Común)

### Concepto
Los WQ son **fungibles** — el usuario puede canjearlos en **cualquier comercio aliado**, no solo donde los ganó.

### Roles de Comercios
1. **Sponsor**: Financia quests para atraer clientes
2. **Punto de Canje**: Acepta WQ como pago
3. **Ambos**: Financia y acepta WQ

### Estructura de Fees
| Operación | Fee | Destino |
|-----------|-----|---------|
| Depósito de comercio | 10% | WanderQuest |
| Recirculación de WQ | 5% | WanderQuest (se queman) |
| Retiro de usuario | 20% | WanderQuest |
| Canje en comercio | 0% | Sin fee para usuario |

### Valores
- **1 WQ ≈ 100 CLP ≈ $0.10 USD**
- Quest típica: 20-50 WQ
- Balance ejemplo: 250 WQ

---

## App Móvil — Detalles

### AppLayout.astro
Layout compartido para todas las páginas de `/app`. Incluye:
- Bottom navigation con 5 tabs (Mapa, Mis Quests, Escanear, Billetera, Perfil)
- FAB central para escanear
- Contenedor `max-w-md` para simular móvil en desktop
- Logo WanderQuest en fondo de desktop
- Props: `activeTab`, `showBottomNav`, `title`

### Páginas

**index.astro (Mapa)**
- Mapa simulado con patrón CSS
- Marcadores de quests (teal, coral para limitados, gold para narrativas)
- Barra de búsqueda
- Filtros: Todos, Narrativa, Limitados, Ofertas Especiales
- Cards horizontales de quests cercanas

**quest.astro (Detalle)**
- Hero con imagen real (Cerro Santa Lucía)
- Badge "Quest Narrativa"
- Stats: distancia, dificultad, duración
- Recompensa: 50 WQ (~$5.000 CLP)
- Sponsor integrado como financiador
- CTA "Iniciar Quest"

**scan.astro (Scanner)**
- Viewfinder con animación de escaneo
- Indicador AR flotante
- Panel de objetivo actual
- Overlay de éxito: "Canjéalos en cualquier comercio aliado"
- Click para toggle entre estados (demo)

**wallet.astro (Billetera)**
- Card de balance: 250 WQ ≈ $25.000 CLP
- Badge "STELLAR NETWORK"
- Botón "Retirar a dinero real (fee 20%)"
- Texto: "Canjeable en todos los comercios aliados"
- Mapa de socios cercanos
- Carrusel de ofertas destacadas
- Historial de transacciones

**profile.astro (Perfil)**
- Card de perfil con avatar y nivel
- Barra de XP
- Balance WQ con equivalente CLP
- Stats: 47 quests, 12 rutas
- Ranking Santiago (#34)
- Grid de logros (3 desbloqueados, 3 bloqueados)
- Quests recientes

---

## Landing Page — Secciones

### Hero.astro
- Vanta.js GLOBE animado
- Badge "Powered by Stellar" con logo real

### JourneyMap.astro
- GSAP ScrollTrigger con pinning
- 90 dashes animados
- 5 checkpoints (paso 5: "canjéalos en cualquier comercio aliado — o retíralos como dinero real")
- 5 floating features (sin "Crea tus quests")

### ForWho.astro
- Sección "Dos mundos": Exploradores (teal) y Negocios (coral)

### Sponsors.astro
- Fondo navy con Vanta.js FOG
- Diagrama de ecosistema: Comercio → Pool Común → Explorador
- Flecha de retorno "canje en cualquier comercio"
- Carrusel de partners con navegación

### BusinessCTA.astro
- Gradiente coral (from-accent to-accent-dark)
- Layout: Formulario izquierda, texto derecha
- Formulario: nombre negocio, contacto, email, tipo
- Beneficios: CPVV, red de comercios, dashboard

### FinalCTA.astro
- Gradiente teal (from-primary to-primary-dark)
- Layout: Texto izquierda, formulario derecha
- Newsletter para usuarios

### Navbar.astro
- Logo real de WanderQuest
- CTA "Descarga la app" → `/app`

### Footer.astro
- Logo real de WanderQuest
- Badge "Powered by Stellar" con logo real
- Links de navegación y redes sociales

---

## Comandos

```bash
# Desarrollo
npm run dev          # Puerto 4322

# Build
npm run build

# Preview
npm run preview
```

---

## Estilos App Móvil (global.css)

```css
/* Animaciones */
.animate-pulse-ring    /* Marcador de ubicación */
.scan-line             /* Línea de escaneo */

/* Utilidades */
.hide-scrollbar        /* Ocultar scrollbar */
.glass-panel           /* Glassmorphism */
.bg-map-pattern        /* Fondo de mapa simulado */

/* Sombras */
--shadow-glass
--shadow-float
--shadow-soft
```

---

## Pendientes / Ideas Futuras

- [x] ~~Mockup de app móvil para video pitch~~ (completado)
- [x] ~~Consistencia económica en pantallas~~ (completado)
- [x] ~~BusinessCTA para comercios~~ (completado)
- [x] ~~Logos reales (WanderQuest, Stellar)~~ (completado)
- [ ] FAQ section
- [ ] Video pitch de 3 minutos
- [ ] Optimizar carga de Vanta (lazy load)

---

## Sesión 5 Mar 2026 — Cambios

1. **Modelo Económico Pool Común**: Documentado y aplicado en todas las pantallas
2. **JourneyMap**: Paso 5 actualizado, eliminado "Crea tus quests"
3. **Sponsors.astro**: Rediseñado con diagrama de ecosistema + Vanta FOG
4. **BusinessCTA.astro**: Nuevo componente CTA para comercios
5. **Logos reales**:
   - `wanderquest-logo.png` (128x128) en Navbar, Footer, AppLayout
   - `stellar-logo-small.png` (48x41, aspect ratio correcto) en Hero, Footer
   - `cerro-santa-lucia-small.png` (400x400) en quest.astro
6. **wallet.astro**: Botón retiro con fee, texto fungibilidad
7. **scan.astro**: Mensaje éxito con "cualquier comercio aliado"
8. **profile.astro**: Balance con equivalente CLP

---

## Para Ver en Modo Móvil

1. Abre http://localhost:4322/app
2. DevTools → Toggle Device Toolbar (F12 → Ctrl+Shift+M)
3. Selecciona iPhone 14 o similar
4. Navega entre pantallas con el bottom nav
