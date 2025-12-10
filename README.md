# 🚊 NaoLaMetric

Affiche les temps d'attente des transports en commun nantais (TAN) sur LaMetric Time.

[![Release](https://github.com/alegeay/naolametric/actions/workflows/release.yml/badge.svg?branch=main)](https://github.com/alegeay/naolametric/actions/workflows/release.yml)
[![PR Pipeline](https://github.com/alegeay/naolametric/actions/workflows/pr_pipeline.yml/badge.svg)](https://github.com/alegeay/naolametric/actions/workflows/pr_pipeline.yml)
[![Rust](https://img.shields.io/badge/Rust-1.83+-orange?logo=rust)](https://www.rust-lang.org/)
[![Docker](https://img.shields.io/badge/Docker-652KB-blue?logo=docker)](https://ghcr.io/alegeay/naolametric)
[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)

![LaMetric Time affichant NaoLaMetric](image.png)

---

## ✨ Caractéristiques

| | Fonctionnalité | Description |
|:--:|----------------|-------------|
| ⚡ | Temps réel | Données live depuis l'API Naolib/TAN |
| 🪶 | Ultra-léger | Image Docker de 652 KB |
| 🚀 | Rapide | ~2ms par requête |
| 💾 | Cache intelligent | 1182 arrêts en mémoire, rafraîchi toutes les heures |

---

## 🏗️ Architecture

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

**Stack :** `tiny_http`, `minreq` (rustls), `serde_json`, musl + UPX

---

## 📦 Installation

### Docker

```bash
docker run -d -p 8080:8080 --name naolametric ghcr.io/music-analysis/naolametric:latest
```

### Docker Compose

```yaml
services:
  naolametric:
    image: ghcr.io/alegeay/NaoLametric:latest
    ports:
      - "8080:8080"
    restart: unless-stopped
```

### Build local

```bash
git clone https://github.com/NaoLametric/naolametric.git
cd naolametric
docker build -t naolametric .
docker run -d -p 8080:8080 naolametric
```

### Cargo

```bash
cargo build --release
./target/release/naolametric
```

---

## 🚀 Utilisation

```bash
# Prochains passages à Commerce
curl "http://localhost:8080/?stop=COMM"

# Ligne 1 direction François Mitterrand
curl "http://localhost:8080/?stop=COMM&line=1&direction=1"

# 5 passages avec destination affichée
curl "http://localhost:8080/?stop=COMM&limit=5&show_terminus=true"
```

Réponse :
```json
{
  "frames": [
    { "icon": "8958", "text": "L1 2mn" },
    { "icon": "8958", "text": "L1 6mn" }
  ]
}
```

---

## 📺 Configuration LaMetric Time

### My Data DIY (simple)

1. Installer l'app **My Data DIY** sur votre LaMetric
2. URL : `http://VOTRE_IP:8080/?stop=COMM&line=1&direction=1`
3. Poll frequency : 30 secondes

### Application personnalisée

1. Créer un compte sur [developer.lametric.com](https://developer.lametric.com)
2. Créer une **Indicator App** en mode **Poll**
3. URL : `http://VOTRE_SERVEUR:8080/?stop={{stop}}&line={{line}}&direction={{direction}}`

---

## 📖 API

### `GET /` — Temps d'attente

| Paramètre | Type | Requis | Description |
|-----------|------|--------|-------------|
| `stop` | string | oui | Code arrêt (ex: `COMM`, `GSNO`) |
| `line` | string | non | Numéro de ligne (ex: `1`, `C1`) |
| `direction` | int | non | Direction : `1` ou `2` |
| `limit` | int | non | Nombre de résultats (1-10) |
| `show_terminus` | bool | non | Afficher la destination |

### Autres endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /stops?search=gare` | Recherche d'arrêts |
| `GET /popular-stops` | Arrêts les plus fréquentés |
| `GET /health` | Health check |
| `GET /info` | Documentation API |

---

## 🚏 Arrêts courants

| Code | Nom | Lignes |
|------|-----|--------|
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

---

## 🎨 Icônes LaMetric

| Type | Lignes | ID |
|------|--------|-----|
| 🚊 Tramway | 1, 2, 3 | 8958 |
| 🚌 Bus | Autres | 7956 |
| ⛴️ Navibus | N1, N2... | 12186 |
| ⚠️ Erreur | — | 555 |

---

## ⚠️ Messages d'erreur

| Message | Cause |
|---------|-------|
| `No stop` | Paramètre `stop` manquant |
| `Bad stop` | Code d'arrêt invalide |
| `Bad dir` | Direction invalide (1 ou 2) |
| `API err` | Erreur API TAN |
| `Aucun` | Aucun passage prévu |

---

## 🛠️ Développement

```bash
cargo run                    # Mode dev
cargo build --release        # Build optimisé
PORT=9090 cargo run          # Autre port
```

---

## 📄 Licence

MIT

## Crédits

- Données : [API Naolib / TAN Nantes](https://open.tan.fr)
- Icônes : [LaMetric Icon Gallery](https://developer.lametric.com/icons)
