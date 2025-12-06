# NaoLaMetric

Affiche les temps d'attente des transports en commun nantais (TAN) sur votre LaMetric Time en temps réel.

[![Rust](https://img.shields.io/badge/Rust-1.83+-orange?logo=rust)](https://www.rust-lang.org/)
[![Docker](https://img.shields.io/badge/Docker-652KB-blue?logo=docker)](https://hub.docker.com/)
[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)
[![CI](https://github.com/votre-repo/naolametric/actions/workflows/ci.yml/badge.svg)](https://github.com/votre-repo/naolametric/actions)

## Aperçu

```
┌─────────────────────────┐
│  🚊  L1 3mn             │
│  🚊  L1 8mn             │
│  🚌  C1 12mn            │
└─────────────────────────┘
```

## Caractéristiques

| Fonctionnalité | Description |
|----------------|-------------|
| **Temps réel** | Données live depuis l'API Naolib/TAN |
| **Ultra-léger** | Image Docker de seulement **652 KB** |
| **Rapide** | Démarrage instantané, ~2ms par requête |
| **Compatible LaMetric** | Format JSON natif pour LaMetric Time |
| **Cache intelligent** | 1182 arrêts en cache, rafraîchi toutes les heures |

## Architecture

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  LaMetric    │────▶│ NaoLaMetric  │────▶│  API TAN     │
│  Time        │◀────│  (Rust)      │◀────│  (Naolib)    │
└──────────────┘     └──────────────┘     └──────────────┘
                            │
                     ┌──────┴──────┐
                     │ Cache arrêts│
                     │ (in-memory) │
                     └─────────────┘
```

**Stack technique :**
- `tiny_http` - Serveur HTTP minimaliste
- `minreq` - Client HTTP avec TLS (rustls)
- `serde_json` - Parsing JSON
- Compilation statique avec musl + compression UPX

## Installation

### Docker (recommandé)

```bash
docker run -d -p 8080:8080 --name naolametric ghcr.io/votre-repo/naolametric:latest
```

### Docker Compose

```yaml
services:
  naolametric:
    image: ghcr.io/votre-repo/naolametric:latest
    ports:
      - "8080:8080"
    restart: unless-stopped
```

### Build local

```bash
git clone https://github.com/votre-repo/naolametric.git
cd naolametric
docker build -t naolametric .
docker run -d -p 8080:8080 naolametric
```

### Cargo (développement)

```bash
cargo build --release
./target/release/naolametric
```

## Utilisation rapide

```bash
# Prochains passages à Commerce
curl "http://localhost:8080/?stop=COMM"

# Ligne 1 direction François Mitterrand
curl "http://localhost:8080/?stop=COMM&line=1&direction=1"

# 5 passages avec destination affichée
curl "http://localhost:8080/?stop=COMM&limit=5&show_terminus=true"
```

**Réponse :**
```json
{
  "frames": [
    { "icon": "8958", "text": "L1 2mn" },
    { "icon": "8958", "text": "L1 6mn" }
  ]
}
```

## Configuration LaMetric Time

### Option 1 : My Data DIY (simple)

1. Installer l'app **My Data DIY** sur votre LaMetric
2. Configurer l'URL :
   ```
   http://VOTRE_IP:8080/?stop=COMM&line=1&direction=1
   ```
3. Poll frequency : **30 secondes**

### Option 2 : Application personnalisée

1. Créer un compte sur [developer.lametric.com](https://developer.lametric.com)
2. Créer une **Indicator App** en mode **Poll**
3. URL : `http://VOTRE_SERVEUR:8080/?stop={{stop}}&line={{line}}&direction={{direction}}`

## API Reference

### `GET /` - Temps d'attente

| Paramètre | Type | Requis | Description |
|-----------|------|--------|-------------|
| `stop` | string | **Oui** | Code arrêt (ex: `COMM`, `GSNO`) |
| `line` | string | Non | Numéro de ligne (ex: `1`, `C1`) |
| `direction` | int | Non | Direction : `1` ou `2` |
| `limit` | int | Non | Nombre de résultats (1-10) |
| `show_terminus` | bool | Non | Afficher la destination |

### `GET /stops` - Recherche d'arrêts

```bash
curl "http://localhost:8080/stops?search=gare&limit=10"
```

### `GET /popular-stops` - Arrêts populaires

Retourne les arrêts les plus fréquentés pour les dropdowns.

### `GET /health` - Health check

Retourne `OK` si le serveur fonctionne.

### `GET /info` - Documentation API

Documentation complète au format JSON.

## Arrêts courants

| Code | Nom | Lignes principales |
|------|-----|-------------------|
| `COMM` | Commerce | 1, 2, 3 |
| `GSNO` | Gare Nord - Jardin des Plantes | 1 |
| `CRQU` | Place du Cirque | 2, 3 |
| `HVNA` | Hôtel de Ville | 1, C1 |
| `OGVA` | Orvault Grand Val | 2 |
| `NETR` | Neustrie | 3 |
| `OTAG` | 50 Otages | 2, 3 |
| `BOFA` | Bouffay | 1 |
| `BJOI` | Beaujoire | 1 |
| `FMIT` | François Mitterrand | 1 |

Rechercher un arrêt : `curl "http://localhost:8080/stops?search=commerce"`

## Variables d'environnement

| Variable | Description | Défaut |
|----------|-------------|--------|
| `PORT` | Port du serveur | `8080` |
| `NAOLIB_STOP_CODE` | Code arrêt par défaut | - |

## Icônes LaMetric

| Type | Lignes | ID Icône |
|------|--------|----------|
| 🚊 Tramway | 1, 2, 3 | 8958 |
| 🚌 Bus | Autres | 7956 |
| ⛴️ Navibus | N1, N2... | 12186 |
| ⚠️ Erreur | - | 555 |

## Messages d'erreur

| Message | Cause |
|---------|-------|
| `No stop` | Paramètre `stop` manquant |
| `Bad stop` | Code d'arrêt invalide |
| `Bad dir` | Direction invalide (1 ou 2) |
| `API err` | Erreur API TAN |
| `Aucun` | Aucun passage prévu |

## Développement

```bash
# Lancer en mode dev
cargo run

# Build release optimisé
cargo build --release

# Lancer sur un autre port
PORT=9090 cargo run
```

## Licence

MIT

## Crédits

- Données temps réel : [API Naolib / TAN Nantes](https://open.tan.fr)
- Icônes : [LaMetric Icon Gallery](https://developer.lametric.com/icons)
