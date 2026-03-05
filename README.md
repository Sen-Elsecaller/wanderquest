# WanderQuest

Plataforma de exploración urbana gamificada. Descubre lugares, completa misiones y gana tokens canjeables en comercios locales.

**Proyecto para Build on Stellar Chile Ideatón 2026**

![Stellar](https://img.shields.io/badge/Powered%20by-Stellar-black?logo=stellar)
![Astro](https://img.shields.io/badge/Built%20with-Astro-ff5d01?logo=astro)

## Concepto

WanderQuest convierte la exploración urbana en un sistema de recompensas verificable:

- **Para usuarios:** Explora tu ciudad, completa quests, verifica tu visita con QR/geolocalización y gana tokens WQ canjeables en comercios aliados.
- **Para comercios:** Marketing basado en visitas reales (CPVV - Cost per Verified Visit), no en métricas infladas.

## Stack Técnico

| Tecnología | Uso |
|------------|-----|
| Astro 5.17 | Framework principal |
| Tailwind CSS v4 | Estilos |
| GSAP + ScrollTrigger | Animaciones scroll-driven |
| Vanta.js | Fondos 3D animados |
| Stellar | Emisión de tokens WQ |

## Demo

- **Landing page:** Presenta el concepto y ecosistema
- **Prototipo app:** 5 pantallas navegables (Mapa, Quest, Scanner, Wallet, Profile)

## Rutas

| Ruta | Descripción |
|------|-------------|
| `/` | Landing page |
| `/app` | Mapa con quests cercanas |
| `/app/quest` | Detalle de quest |
| `/app/scan` | Scanner QR |
| `/app/wallet` | Billetera WQ |
| `/app/profile` | Perfil de usuario |

## Desarrollo

```bash
# Instalar dependencias
npm install

# Servidor de desarrollo
npm run dev

# Build
npm run build
```

## Modelo Económico

- **1 WQ ≈ 100 CLP** (tipo de cambio fijo)
- Tokens fungibles canjeables en cualquier comercio aliado
- Opción de retiro a dinero real (fee 20%)
- Comercios pagan solo por visitas verificadas

## Equipo

Proyecto desarrollado para el Ideatón Build on Stellar Chile 2026.

---

*Powered by Stellar*
