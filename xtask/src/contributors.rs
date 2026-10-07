//! `cargo xtask contributors`: refresh contributors/commits.tsv and prs.tsv from the local git
//! history and GitHub (via `gh`), per craftrules standards/contributors.md. Never writes
//! people.tsv or models.tsv. Git author names and emails are never recorded.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

const REPO: &str = "storytold/cadcraft";

fn run(root: &Path, cmd: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(cmd).current_dir(root).args(args).output().map_err(|e| format!("{cmd}: {e}"))?;
    if !out.status.success() {
        return Err(format!("{cmd} {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn trailer_names(models: &str) -> HashMap<String, String> {
    models
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("id\t"))
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            Some((c.get(4)?.to_string(), c.first()?.to_string()))
        })
        .collect()
}

pub fn run_cmd(root: &Path) -> Result<(), String> {
    let models = std::fs::read_to_string(root.join("contributors/models.tsv")).unwrap_or_default();
    let map = trailer_names(&models);
    // Commit → GitHub login (paginated; the only network step).
    let mut logins: HashMap<String, String> = HashMap::new();
    for page in 1..=200 {
        let path = format!("repos/{REPO}/commits?per_page=100&page={page}");
        let Ok(out) = run(root, "gh", &["api", &path, "--jq", ".[] | [.sha, (.author.login // \"unknown\")] | @tsv"]) else { break };
        if out.trim().is_empty() {
            break;
        }
        for l in out.lines() {
            if let Some((sha, login)) = l.split_once('\t') {
                logins.insert(sha.to_string(), login.to_string());
            }
        }
    }
    // PR numbers by merge commit.
    let mut prs_rows = Vec::new();
    let mut pr_of: HashMap<String, String> = HashMap::new();
    if let Ok(out) = run(
        root,
        "gh",
        &[
            "pr",
            "list",
            "--repo",
            REPO,
            "--state",
            "merged",
            "--limit",
            "1000",
            "--json",
            "number,url,author,mergedAt,additions,deletions,commits,mergeCommit",
            "--jq",
            ".[] | [.number, .url, .author.login, .mergedAt, .additions, .deletions, (.commits|length), .mergeCommit.oid, ([.commits[].oid]|join(\",\"))] | @tsv",
        ],
    ) {
        for l in out.lines() {
            let c: Vec<&str> = l.split('\t').collect();
            if c.len() < 9 {
                continue;
            }
            for sha in c[8].split(',') {
                pr_of.insert(sha.to_string(), c[0].to_string());
            }
            prs_rows.push(c[..7].join("\t"));
        }
    }
    // Local history with numstat and trailers.
    let out = Command::new("git")
        .current_dir(root)
        .env("TZ", "UTC")
        .args([
            "log",
            "--no-merges",
            "--date=format-local:%Y-%m-%dT%H:%M:%SZ",
            "--format=@@%H\t%cd\t%(trailers:key=Co-Authored-By,valueonly,separator=;)",
            "--numstat",
        ])
        .output()
        .map_err(|e| format!("git: {e}"))?;
    let log = String::from_utf8_lossy(&out.stdout).to_string();
    let mut rows: Vec<(String, String)> = Vec::new();
    let mut cur: Option<(String, String, String, u64, u64)> = None;
    let flush = |cur: &mut Option<(String, String, String, u64, u64)>, rows: &mut Vec<(String, String)>| {
        if let Some((sha, date, trailers, add, del)) = cur.take() {
            let login = logins.get(&sha).cloned().unwrap_or_else(|| "unknown".into());
            let mut ms: Vec<String> = trailers
                .split(';')
                .filter_map(|t| {
                    let name = t.trim().split('<').next()?.trim().to_string();
                    map.get(&name).cloned()
                })
                .collect();
            ms.sort();
            ms.dedup();
            let pr = pr_of.get(&sha).cloned().unwrap_or_default();
            rows.push((
                date.clone(),
                format!("{sha}\thttps://github.com/{REPO}/commit/{sha}\t{login}\t{date}\t{add}\t{del}\t{pr}\t{}", ms.join(";")),
            ));
        }
    };
    for l in log.lines() {
        if let Some(rest) = l.strip_prefix("@@") {
            flush(&mut cur, &mut rows);
            let c: Vec<&str> = rest.splitn(3, '\t').collect();
            cur = Some((c.first().unwrap_or(&"").to_string(), c.get(1).unwrap_or(&"").to_string(), c.get(2).unwrap_or(&"").to_string(), 0, 0));
        } else if let Some(c) = cur.as_mut() {
            let p: Vec<&str> = l.split('\t').collect();
            if p.len() == 3 {
                c.3 += p[0].parse::<u64>().unwrap_or(0);
                c.4 += p[1].parse::<u64>().unwrap_or(0);
            }
        }
    }
    flush(&mut cur, &mut rows);
    rows.sort();
    let mut commits = String::from("sha\turl\tlogin\tdate\tadditions\tdeletions\tpr\tmodels\n");
    for (_, r) in &rows {
        commits.push_str(r);
        commits.push('\n');
    }
    std::fs::write(root.join("contributors/commits.tsv"), commits).map_err(|e| e.to_string())?;
    let mut prs = String::from("number\turl\tlogin\tmerged_at\tadditions\tdeletions\tcommits\tmodels\n");
    prs_rows.sort();
    for r in prs_rows {
        prs.push_str(&r);
        prs.push_str("\t\n");
    }
    std::fs::write(root.join("contributors/prs.tsv"), prs).map_err(|e| e.to_string())?;
    println!("contributors: {} commits", rows.len());
    Ok(())
}
