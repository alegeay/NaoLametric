# NaoLaMetric

Affiche les temps d'attente des transports en commun nantais (TAN) sur votre LaMetric Time en temps réel.

![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Apercu

```
┌─────────────────────────┐
│  🚊  L1 3mn             │
│  🚊  L1 8mn             │
└─────────────────────────┘
```

## Fonctionnalités

- Temps d'attente en temps réel depuis l'API Naolib/TAN
- Filtrage par ligne et direction
- Icônes adaptées (tramway, bus, navibus)
- Validation des codes d'arrêt
- Configuration via URL (compatible LaMetric)
- Cache intelligent des arrêts

## Installation

### Docker (recommandé)

```bash
git clone https://github.com/votre-repo/naolametric.git
cd naolametric
docker-compose up -d
```

### Cargo

```bash
cargo build --release
./target/release/naolametric
```

Le serveur démarre sur le port `8080` par défaut.

## Utilisation rapide

### Exemples de requêtes

```bash
# Prochains passages à Commerce
curl "http://localhost:8080/?stop=COMM"

# Ligne 1 à Souillarderie vers François Mitterrand
curl "http://localhost:8080/?stop=SOUI&line=1&direction=1"

# Avec la destination affichée
curl "http://localhost:8080/?stop=SOUI&line=1&direction=1&show_terminus=true"

# 5 prochains passages à Gare de Nantes
curl "http://localhost:8080/?stop=GANO&limit=5"
```

### Réponse LaMetric

```json
{
  "frames": [
    { "icon": "i8958", "text": "L1 3mn" },
    { "icon": "i8958", "text": "L1 8mn" }
  ]
}
```

## Configuration LaMetric Time

### Option 1 : My Data DIY (simple)

1. Ouvrir l'app **LaMetric Time** sur votre smartphone
2. Aller dans la bibliothèque d'apps
3. Installer **My Data DIY**
4. Configurer :
   - **URL** : `http://VOTRE_IP:8080/?stop=SOUI&line=1&direction=1`
   - **Poll frequency** : 30 secondes

### Option 2 : Application personnalisée (avancé)

1. Créer un compte sur [developer.lametric.com](https://developer.lametric.com)
2. Créer une **Indicator App** en mode **Poll**
3. URL de polling :
   ```
   http://VOTRE_SERVEUR:8080/?stop={{stop}}&line={{line}}&direction={{direction}}&show_terminus={{show_terminus}}
   ```
4. Ajouter les champs utilisateur :

| Nom affiché | ID | Type | Options |
|-------------|-----|------|---------|
| Arrêt | `stop` | Dropdown | `COMM:Commerce`, `GANO:Gare de Nantes`, `SOUI:Souillarderie`... |
| Ligne | `line` | Text | *(optionnel)* |
| Direction | `direction` | Dropdown | `1:Aller`, `2:Retour` |
| Afficher destination | `show_terminus` | Checkbox | |

5. Fréquence de poll : **30 secondes**

## API Reference

### `GET /` - Temps d'attente

Retourne les prochains passages formatés pour LaMetric.

| Paramètre | Type | Requis | Description |
|-----------|------|--------|-------------|
| `stop` | string | **Oui** | Code de l'arrêt (ex: `COMM`, `SOUI`) |
| `line` | string | Non | Numéro de ligne (ex: `1`, `2`, `C1`) |
| `direction` | integer | Non | Direction : `1` ou `2` |
| `limit` | integer | Non | Nombre de résultats (1-10, défaut: 2) |
| `show_terminus` | boolean | Non | Afficher la destination (défaut: false) |

**Exemple :**
```bash
curl "http://localhost:8080/?stop=SOUI&line=1&direction=1"
```

**Réponse :**
```json
{
  "frames": [
    { "icon": "i8958", "text": "L1 3mn" },
    { "icon": "i8958", "text": "L1 8mn" }
  ]
}
```

### `GET /stops` - Recherche d'arrêts

Recherche parmi tous les arrêts du réseau TAN.

| Paramètre | Type | Description |
|-----------|------|-------------|
| `search` | string | Terme de recherche |
| `limit` | integer | Limite de résultats (défaut: 100) |

**Exemple :**
```bash
curl "http://localhost:8080/stops?search=commerce"
```

**Réponse :**
```json
[
  { "codeLieu": "COMM", "libelle": "Commerce" }
]
```

### `GET /popular-stops` - Arrêts populaires

Liste des arrêts les plus fréquentés (pour dropdown).

```bash
curl "http://localhost:8080/popular-stops"
```

```json
[
  { "code": "COMM", "name": "Commerce" },
  { "code": "GANO", "name": "Gare de Nantes" },
  { "code": "SOUI", "name": "Souillarderie" }
]
```

### `GET /info` - Documentation API

Retourne la documentation complète en JSON.

### `GET /health` - Health check

Retourne `OK` si le serveur fonctionne.

## Trouver son arrêt

### Méthode 1 : Recherche via l'API

```bash
# Chercher un arrêt contenant "gare"
curl "http://localhost:8080/stops?search=gare"
```

### Méthode 2 : Liste officielle TAN

Consulter : https://open.tan.fr/ewp/arrets.json

### Arrêts courants

| Code | Nom | Lignes |
|------|-----|--------|
| `COMM` | Commerce | 1, 2, 3 |
| `GANO` | Gare de Nantes | 1, C1, C6 |
| `SOUI` | Souillarderie | 1 |
| `CRQU` | Place du Cirque | 2, 3 |
| `MEDI` | Médiathèque | 1 |
| `HBLI` | Hôtel de Ville | 1, C1 |
| `CICE` | Cité des Congrès | 1, C1 |
| `5050` | 50 Otages | 2, 3 |

## Trouver la bonne direction

La direction dépend de l'arrêt et de la ligne. Pour la trouver :

```bash
# Afficher tous les passages avec leur destination
curl "http://localhost:8080/?stop=SOUI&show_terminus=true&limit=10"
```

Résultat :
```json
{
  "frames": [
    { "text": "1 François M. 3mn" },   // direction=1
    { "text": "1 Jamet 6mn" },          // direction=1
    { "text": "1 Beaujoire 7mn" },      // direction=2
    { "text": "1 Babinière 14mn" }      // direction=2
  ]
}
```

Puis tester :
```bash
# Direction 1 = François Mitterrand
curl "http://localhost:8080/?stop=SOUI&line=1&direction=1"

# Direction 2 = Beaujoire
curl "http://localhost:8080/?stop=SOUI&line=1&direction=2"
```

## Variables d'environnement

| Variable | Description | Défaut |
|----------|-------------|--------|
| `PORT` | Port du serveur | `8080` |
| `NAOLIB_STOP_CODE` | Code arrêt par défaut | *(aucun)* |
| `NAOLIB_LINE` | Ligne par défaut | *(aucun)* |
| `NAOLIB_DIRECTION` | Direction par défaut | *(aucun)* |
| `NAOLIB_LIMIT` | Nombre de résultats | `2` |

Exemple `.env` :
```env
PORT=8080
NAOLIB_STOP_CODE=SOUI
NAOLIB_LINE=1
NAOLIB_DIRECTION=1
```

## Docker Compose

```yaml
version: '3.8'
services:
  naolametric:
    build: .
    ports:
      - "8080:8080"
    environment:
      - PORT=8080
    restart: unless-stopped
```

## Messages d'erreur

| Affichage | Cause |
|-----------|-------|
| `No stop` | Paramètre `stop` manquant |
| `Bad stop` | Code d'arrêt invalide |
| `Bad dir` | Direction invalide (doit être 1 ou 2) |
| `API err` | Erreur de l'API TAN |
| `Aucun` | Aucun passage prévu |

## Icônes

| Type | Lignes | Icône |
|------|--------|-------|
| Tramway | 1, 2, 3 | i8958 |
| Bus | Autres | i7956 |
| Navibus | N1, N2... | i12186 |
| Erreur | - | i555 |

## Développement

```bash
# Mode développement
cargo run

# Tests
cargo test

# Build release
cargo build --release

# Lancer sur un port différent
PORT=9090 cargo run
```

## Licence

MIT

## Crédits

- Données temps réel : [API Naolib / TAN Nantes](https://open.tan.fr)
- Icônes : [LaMetric Icon Gallery](https://developer.lametric.com/icons)
