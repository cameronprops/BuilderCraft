//! The About window's credits: contributors (GitHub logins; names only with recorded consent)
//! and AI models, from the TSVs compiled in by build.rs.

use std::collections::BTreeMap;

const COMMITS: &str = include_str!(concat!(env!("OUT_DIR"), "/commits.tsv"));
const PRS: &str = include_str!(concat!(env!("OUT_DIR"), "/prs.tsv"));
const MODELS: &str = include_str!(concat!(env!("OUT_DIR"), "/models.tsv"));
const PEOPLE: &str = include_str!(concat!(env!("OUT_DIR"), "/people.tsv"));

fn rows(t: &str) -> Vec<Vec<&str>> {
    t.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')).skip(1).map(|l| l.split('\t').collect()).collect()
}

#[derive(Clone, Debug, Default)]
pub struct Person {
    pub login: String,
    pub display: Option<String>,
    pub real: Option<String>,
    pub first: String,
    pub last: String,
    pub commits: usize,
    pub prs: usize,
    pub added: u64,
    pub removed: u64,
}

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub id: String,
    pub company: String,
    pub model: String,
    pub version: String,
    pub commits: usize,
    pub added: u64,
    pub removed: u64,
}

pub fn people() -> Vec<Person> {
    let mut by: BTreeMap<String, Person> = BTreeMap::new();
    for r in rows(COMMITS) {
        let (Some(login), Some(date)) = (r.get(2), r.get(3)) else { continue };
        let p = by.entry(login.to_string()).or_insert_with(|| Person {
            login: login.to_string(),
            first: date.to_string(),
            last: date.to_string(),
            ..Default::default()
        });
        p.commits += 1;
        p.added += r.get(4).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        p.removed += r.get(5).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        if *date < p.first.as_str() {
            p.first = date.to_string();
        }
        if *date > p.last.as_str() {
            p.last = date.to_string();
        }
    }
    for r in rows(PRS) {
        if let Some(p) = r.get(2).and_then(|l| by.get_mut(*l)) {
            p.prs += 1;
        }
    }
    for r in rows(PEOPLE) {
        if r.get(1) == Some(&"yes")
            && let Some(p) = r.first().and_then(|l| by.get_mut(*l))
        {
            p.display = r.get(4).filter(|s| !s.is_empty()).map(|s| s.to_string());
            p.real = r.get(5).filter(|s| !s.is_empty()).map(|s| s.to_string());
        }
    }
    let mut v: Vec<Person> = by.into_values().filter(|p| !p.login.ends_with("[bot]")).collect();
    v.sort_by(|a, b| a.first.cmp(&b.first));
    v
}

pub fn models() -> (Vec<Model>, usize) {
    let meta: BTreeMap<String, (String, String, String)> = rows(MODELS)
        .iter()
        .filter_map(|r| Some((r.first()?.to_string(), (r.get(1)?.to_string(), r.get(2)?.to_string(), r.get(3).unwrap_or(&"").to_string()))))
        .collect();
    let mut by: BTreeMap<String, Model> = BTreeMap::new();
    let all = rows(COMMITS);
    for r in &all {
        for id in r.get(7).unwrap_or(&"").split(';').filter(|s| !s.is_empty()) {
            let (company, model, version) = meta.get(id).cloned().unwrap_or_default();
            let m = by.entry(id.to_string()).or_insert_with(|| Model { id: id.to_string(), company, model, version, ..Default::default() });
            m.commits += 1;
            m.added += r.get(4).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            m.removed += r.get(5).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        }
    }
    (by.into_values().collect(), all.len())
}
