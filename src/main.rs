// NaoLaMetric - Version ultra-optimisée (stack synchrone)
// Serveur HTTP minimaliste pour LaMetric Time

use serde::{Deserialize, Serialize};
use std::env;
use std::sync::{LazyLock, RwLock};
use std::time::Instant;
use tiny_http::{Header, Method, Response, Server};

// ============================================================================
// Constantes
// ============================================================================

const ICONE_TRAM: &str = "8958";
const ICONE_BUS: &str = "7956";
const ICONE_BATEAU: &str = "12186";
const ICONE_ERREUR: &str = "555";
const CACHE_TTL_SECS: u64 = 3600;
const HTTP_TIMEOUT_SECS: u64 = 10;
const API_URL: &str = "https://open.tan.fr/ewp";

// Arrêts populaires - tableau compile-time
const ARRETS_POPULAIRES: &str = r#"[
{"code":"COMM","name":"Commerce"},
{"code":"GSNO","name":"Gare Nord - Jardin des Plantes"},
{"code":"CRQU","name":"Place du Cirque"},
{"code":"HVNA","name":"Hôtel de Ville"},
{"code":"OGVA","name":"Orvault Grand Val"},
{"code":"NETR","name":"Neustrie"},
{"code":"VTOU","name":"Vertou"},
{"code":"SDON","name":"St-Donatien"},
{"code":"OTAG","name":"50 Otages"},
{"code":"BOFA","name":"Bouffay"},
{"code":"DCAN","name":"Duchesse Anne - Château"},
{"code":"BJOI","name":"Beaujoire"},
{"code":"FMIT","name":"François Mitterrand"},
{"code":"HALU","name":"Haluchère - Batignolles"}
]"#;

// Header Content-Type JSON pré-alloué
static JSON_HEADER: LazyLock<Header> = LazyLock::new(|| {
    Header::from_bytes(
        &b"Content-Type"[..],
        &b"application/json; charset=utf-8"[..],
    )
    .unwrap()
});

// ============================================================================
// Fonctions HTTP (minreq - ultra-minimal)
// ============================================================================

fn http_get_json<T: serde::de::DeserializeOwned>(
    url: &str,
) -> Result<T, Box<dyn std::error::Error>> {
    let response = minreq::get(url).with_timeout(HTTP_TIMEOUT_SECS).send()?;
    Ok(serde_json::from_str(response.as_str()?)?)
}

// ============================================================================
// Cache des arrêts
// ============================================================================

struct CacheArrets {
    liste: Vec<ArretNaolib>,
    derniere_maj: Option<Instant>,
}

static CACHE_ARRETS: LazyLock<RwLock<CacheArrets>> = LazyLock::new(|| {
    RwLock::new(CacheArrets {
        liste: Vec::new(),
        derniere_maj: None,
    })
});

fn rafraichir_cache() -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{API_URL}/arrets.json");
    let arrets: Vec<ArretNaolib> = http_get_json(&url)?;

    if let Ok(mut cache) = CACHE_ARRETS.write() {
        eprintln!("[INFO] Cache rafraîchi : {} arrêts", arrets.len());
        cache.liste = arrets;
        cache.derniere_maj = Some(Instant::now());
    }

    Ok(())
}

#[inline]
fn cache_valide() -> bool {
    CACHE_ARRETS
        .read()
        .ok()
        .and_then(|c| c.derniere_maj)
        .is_some_and(|t| t.elapsed().as_secs() < CACHE_TTL_SECS)
}

fn assurer_cache_frais() {
    if !cache_valide()
        && let Err(e) = rafraichir_cache()
    {
        eprintln!("[WARN] Échec rafraîchissement cache : {}", e);
    }
}

#[inline]
fn code_arret_valide(code: &str) -> bool {
    CACHE_ARRETS.read().ok().is_none_or(|c| {
        c.liste.is_empty()
            || c.liste
                .iter()
                .any(|a| a.code_lieu.eq_ignore_ascii_case(code))
    })
}

// ============================================================================
// Structures API Naolib
// ============================================================================

#[derive(Deserialize)]
struct LigneNaolib {
    #[serde(rename = "numLigne")]
    num_ligne: String,
}

#[derive(Deserialize)]
struct PassageNaolib {
    sens: u8,
    terminus: String,
    temps: String,
    ligne: LigneNaolib,
}

#[derive(Deserialize)]
struct ArretNaolib {
    #[serde(rename = "codeLieu")]
    code_lieu: String,
    libelle: String,
}

// ============================================================================
// Réponse LaMetric
// ============================================================================

#[derive(Serialize)]
struct FrameLaMetric {
    icon: &'static str,
    text: String,
}

#[derive(Serialize)]
struct ReponseLaMetric {
    frames: Vec<FrameLaMetric>,
}

impl ReponseLaMetric {
    fn erreur(message: &'static str) -> String {
        format!(r#"{{"frames":[{{"icon":"{ICONE_ERREUR}","text":"{message}"}}]}}"#)
    }

    fn simple(icone: &'static str, texte: &str) -> String {
        format!(r#"{{"frames":[{{"icon":"{icone}","text":"{texte}"}}]}}"#)
    }
}

// ============================================================================
// Parsing URL simple (sans dépendance)
// ============================================================================

struct Params {
    stop: Option<String>,
    line: Option<String>,
    direction: Option<u8>,
    limit: usize,
    show_terminus: bool,
    search: Option<String>,
}

fn parse_query(query: &str) -> Params {
    let mut params = Params {
        stop: None,
        line: None,
        direction: None,
        limit: 2,
        show_terminus: false,
        search: None,
    };

    for part in query.split('&') {
        if let Some((key, value)) = part.split_once('=') {
            let value = urlencoding::decode(value).unwrap_or_default();
            match key {
                "stop" => params.stop = Some(value.to_uppercase()),
                "line" => params.line = Some(value.to_uppercase()),
                "direction" => params.direction = value.parse().ok(),
                "limit" => params.limit = value.parse().unwrap_or(2).clamp(1, 10),
                "show_terminus" => params.show_terminus = value == "true" || value == "1",
                "search" => params.search = Some(value.to_lowercase()),
                _ => {}
            }
        }
    }

    params
}

// ============================================================================
// Logique métier
// ============================================================================

fn recuperer_passages(code_arret: &str) -> Result<Vec<PassageNaolib>, Box<dyn std::error::Error>> {
    let url = format!("{API_URL}/tempsattente.json/{code_arret}");
    http_get_json(&url)
}

#[inline]
fn icone_ligne(ligne: &str) -> &'static str {
    match ligne.as_bytes().first() {
        Some(b'1'..=b'3') if ligne.len() == 1 => ICONE_TRAM,
        Some(b'N') => ICONE_BATEAU,
        _ => ICONE_BUS,
    }
}

fn formater_reponse(passages: Vec<PassageNaolib>, params: &Params) -> String {
    let filtres: Vec<_> = passages
        .into_iter()
        .filter(|p| {
            !p.temps.is_empty()
                && params
                    .line
                    .as_ref()
                    .is_none_or(|l| p.ligne.num_ligne.eq_ignore_ascii_case(l))
                && params.direction.is_none_or(|d| p.sens == d)
        })
        .take(params.limit)
        .collect();

    if filtres.is_empty() {
        return ReponseLaMetric::simple(ICONE_TRAM, "Aucun");
    }

    let frames: Vec<FrameLaMetric> = filtres
        .into_iter()
        .map(|p| {
            let text = if params.show_terminus {
                let terminus = if p.terminus.len() > 12 {
                    format!("{}.", &p.terminus[..11])
                } else {
                    p.terminus
                };
                format!("{} {terminus} {}", p.ligne.num_ligne, p.temps)
            } else {
                format!("L{} {}", p.ligne.num_ligne, p.temps)
            };
            FrameLaMetric {
                icon: icone_ligne(&p.ligne.num_ligne),
                text,
            }
        })
        .collect();

    serde_json::to_string(&ReponseLaMetric { frames })
        .unwrap_or_else(|_| ReponseLaMetric::erreur("JSON err"))
}

// ============================================================================
// Handlers
// ============================================================================

fn handle_principal(params: &Params) -> (u16, String) {
    // Vérifier arrêt
    let code_arret = match &params.stop {
        Some(stop) if !stop.is_empty() => stop,
        _ => &env::var("NAOLIB_STOP_CODE").unwrap_or_default(),
    };

    if code_arret.is_empty() {
        return (400, ReponseLaMetric::erreur("No stop"));
    }

    assurer_cache_frais();

    if !code_arret_valide(code_arret) {
        return (400, ReponseLaMetric::erreur("Bad stop"));
    }

    if let Some(dir) = params.direction
        && dir != 1
        && dir != 2
    {
        return (400, ReponseLaMetric::erreur("Bad dir"));
    }

    match recuperer_passages(code_arret) {
        Ok(passages) => (200, formater_reponse(passages, params)),
        Err(e) => {
            eprintln!("[ERROR] API Naolib : {e}");
            (502, ReponseLaMetric::erreur("API err"))
        }
    }
}

fn handle_stops(params: &Params) -> (u16, String) {
    assurer_cache_frais();

    let cache = match CACHE_ARRETS.read() {
        Ok(c) => c,
        Err(_) => return (500, r#"{"error":"Cache error"}"#.to_string()),
    };

    if cache.liste.is_empty() {
        return (503, r#"{"error":"Cache not ready"}"#.to_string());
    }

    let limit = params.limit.min(500);
    let mut result = String::with_capacity(4096);
    result.push('[');

    let mut count = 0;
    for ArretNaolib { code_lieu, libelle } in &cache.liste {
        if let Some(search) = &params.search
            && !libelle.to_lowercase().contains(search)
            && !code_lieu.to_lowercase().contains(search)
        {
            continue;
        }

        if count > 0 {
            result.push(',');
        }
        result.push_str(&format!(
            r#"{{"codeLieu":"{code_lieu}","libelle":"{libelle}"}}"#,
        ));
        count += 1;

        if count >= limit {
            break;
        }
    }

    result.push(']');
    (200, result)
}

fn handle_info() -> String {
    format!(
        r#"{{"name":"NaoLaMetric","version":"{}","description":"Application LaMetric pour les transports nantais (TAN/Naolib)","endpoints":[{{"path":"/","method":"GET","description":"Prochains passages pour LaMetric"}},{{"path":"/stops","method":"GET","description":"Recherche d'arrêts"}},{{"path":"/popular-stops","method":"GET","description":"Arrêts populaires"}},{{"path":"/health","method":"GET","description":"État du serveur"}},{{"path":"/info","method":"GET","description":"Documentation API"}}],"parameters":[{{"name":"stop","type":"string","required":true,"description":"Code arrêt (COMM, GANO...)"}},{{"name":"line","type":"string","required":false,"description":"Filtre ligne (1, 2, C1...)"}},{{"name":"direction","type":"integer","required":false,"description":"Direction (1 ou 2)"}},{{"name":"limit","type":"integer","required":false,"description":"Nombre résultats (1-10)"}},{{"name":"show_terminus","type":"boolean","required":false,"description":"Afficher destination"}}],"examples":[{{"description":"Passages Commerce","url":"/?stop=COMM"}},{{"description":"Ligne 1 direction 1","url":"/?stop=COMM&line=1&direction=1"}},{{"description":"5 passages + terminus","url":"/?stop=GANO&limit=5&show_terminus=true"}},{{"description":"Recherche gare","url":"/stops?search=gare"}}]}}"#,
        env!("CARGO_PKG_VERSION")
    )
}

// ============================================================================
// Point d'entrée
// ============================================================================

fn main() {
    // Charger variables d'environnement depuis .env si présent
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');
                if !key.is_empty() && !key.starts_with('#') {
                    // SAFETY: Un seul thread en cours d'exécution ici
                    unsafe { env::set_var(key, value) };
                }
            }
        }
    }

    eprintln!("[INFO] Chargement du cache...");
    if let Err(e) = rafraichir_cache() {
        eprintln!("[WARN] Échec chargement cache : {e}");
    }

    let port = env::var("PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("0.0.0.0:{port}");

    let server = Server::http(&addr).expect("Impossible de démarrer le serveur");
    eprintln!("[INFO] Serveur démarré sur {addr}");

    for request in server.incoming_requests() {
        let url = request.url().to_string();
        let (path, query) = url.split_once('?').unwrap_or((&url, ""));

        let (status, body) = if *request.method() != Method::Get {
            (405, r#"{"error":"Method not allowed"}"#.to_string())
        } else {
            match path {
                "/" => {
                    let params = parse_query(query);
                    handle_principal(&params)
                }
                "/health" => (200, "OK".to_string()),
                "/stops" => {
                    let params = parse_query(query);
                    handle_stops(&params)
                }
                "/popular-stops" => (200, ARRETS_POPULAIRES.to_string()),
                "/info" => (200, handle_info()),
                _ => (404, r#"{"error":"Not found"}"#.to_string()),
            }
        };

        let response = Response::from_string(&body)
            .with_status_code(status)
            .with_header(JSON_HEADER.clone());

        let _ = request.respond(response);
    }
}
