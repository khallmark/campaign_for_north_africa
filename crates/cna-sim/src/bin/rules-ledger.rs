//! Coverage checking only: evidence content and rule interpretation need review.
#[path = "rules-ledger/index.rs"]
mod index;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const HELP: &str = "rules-ledger --check [--section N] [--require STATUS,...] [--check-pages]
rules-ledger --check-pages
rules-ledger --rebuild-index --check-index
rules-ledger --check-index [--data-dir PATH]

Checks are read-only; only --rebuild-index writes files. No generated ledger rows.
  --rebuild-index   Rebuild data/rules/case-index.json and docs/rules/coverage.md
                    from pinned extracts; cannot combine with --data-dir
  --check-index     Check extract hashes, source-lock fingerprints and index/coverage
  --section N       Check completeness/status in section 1–65 (default all)
  --require LIST    Minimum ordinary status; not_applicable is a separate opt-in
  --check-pages     Check image-only page records; report explicit pending pages
  --data-dir PATH   Input directory (default repository data/rules)
  --help            Show help without reading inputs

Statuses: unread, compared, fixtures, implemented, verified, not_applicable
Inputs: case-index.json, ledger.json; evidence-index.json for --check-pages
Examples:
  cargo run -p cna-sim --bin rules-ledger -- --rebuild-index --check-index
  cargo run -p cna-sim --bin rules-ledger -- --check --section 8 --require verified
  cargo run -p cna-sim --bin rules-ledger -- --check --require verified,not_applicable --check-pages
  rules-ledger --check --data-dir /tmp/review-fixture
Exit 0 means requested checks pass, 1 means invalid input or failed checks.
Pending page records are accounted for, NOT certified transcriptions.";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
enum Book {
    L21,
    AL21,
    SC21,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Status {
    Unread,
    Compared,
    Fixtures,
    Implemented,
    Verified,
    NotApplicable,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Case {
    case: String,
    section: u8,
    book: Book,
    page: u16,
    // Preserve source spelling when an explicit indexing alias supplies the key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    raw_label: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    case: String,
    section: u8,
    book: Book,
    page: u16,
    original_ref: String,
    correction_refs: Vec<String>,
    july_text_ref: String,
    #[serde(default, deserialize_with = "present_value")]
    vassal_refs: Option<Vec<String>>,
    #[serde(default, deserialize_with = "present_value")]
    vassal: Option<String>,
    discrepancy: String,
    adjudication: Option<String>,
    status: Status,
    justification: Option<String>,
    code_refs: Vec<String>,
    tests: Vec<String>,
}

// Missing optional VASSAL fields are allowed; explicit null is not schema-valid.
fn present_value<'de, T: Deserialize<'de>, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

fn location(book: Book, page: u16, section: u8) -> bool {
    match book {
        Book::L21 => (1..=48).contains(&page) && (1..=32).contains(&section),
        Book::AL21 => (1..=28).contains(&page) && (33..=58).contains(&section),
        Book::SC21 => (1..=18).contains(&page) && (59..=65).contains(&section),
    }
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn check(index: &[Case], rows: &[Row], section: Option<u8>, required: &[Status]) -> Result<usize> {
    let mut cases = BTreeMap::new();
    for case in index {
        if !location(case.book, case.page, case.section)
            || case
                .case
                .split('.')
                .next()
                .and_then(|s| s.parse::<u8>().ok())
                != Some(case.section)
            || case.raw_label.as_deref().is_some_and(|s| !nonempty(s))
        {
            return Err(format!("invalid index case/location: {}", case.case).into());
        }
        if cases.insert(&case.case, case).is_some() {
            return Err(format!("duplicate index case: {}", case.case).into());
        }
    }
    let selected = |n| section.is_none_or(|s| s == n);
    let expected = index.iter().filter(|c| selected(c.section)).count();
    if expected == 0 {
        return Err(
            "no indexed cases in requested scope; generate/review the case index first".into(),
        );
    }
    let mut present = BTreeMap::new();
    for row in rows {
        let source = cases
            .get(&row.case)
            .ok_or_else(|| format!("unknown ledger case: {}", row.case))?;
        if present.insert(&row.case, row).is_some() {
            return Err(format!("duplicate ledger case: {}", row.case).into());
        }
        if (row.section, row.book, row.page) != (source.section, source.book, source.page) {
            return Err(format!("source location mismatch: {}", row.case).into());
        }
        let refs_valid = [&row.correction_refs, &row.code_refs, &row.tests]
            .into_iter()
            .all(|refs| refs.iter().all(|s| nonempty(s)));
        let vassal_valid = match (&row.vassal, &row.vassal_refs) {
            (Some(absent), None) => absent == "absent",
            (None, Some(refs)) => !refs.is_empty() && refs.iter().all(|s| nonempty(s)),
            _ => false,
        };
        if !refs_valid
            || !vassal_valid
            || ![&row.original_ref, &row.july_text_ref, &row.discrepancy]
                .into_iter()
                .all(|s| nonempty(s))
            || row.adjudication.as_deref().is_some_and(|s| !nonempty(s))
            || row.justification.as_deref().is_some_and(|s| !nonempty(s))
            || (row.status == Status::NotApplicable && row.justification.is_none())
        {
            return Err(format!(
                "invalid evidence fields or missing not_applicable justification: {}",
                row.case
            )
            .into());
        }
        if selected(row.section) && !required.is_empty() {
            let allowed = if row.status == Status::NotApplicable {
                required.contains(&Status::NotApplicable)
            } else {
                required
                    .iter()
                    .any(|r| *r != Status::NotApplicable && row.status >= *r)
            };
            if !allowed {
                return Err(format!(
                    "case {} has status {:?}, below/outside required {:?}",
                    row.case, row.status, required
                )
                .into());
            }
        }
    }
    for case in index.iter().filter(|c| selected(c.section)) {
        if !present.contains_key(&case.case) {
            return Err(format!("missing ledger case: {}", case.case).into());
        }
    }
    Ok(expected)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceIndex {
    image_only_pages: Vec<PageEvidence>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PageEvidence {
    book: Book,
    page: u16,
    status: PageStatus,
    transcript: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum PageStatus {
    Pending,
    Transcribed,
}

fn check_pages(evidence: EvidenceIndex, root: &Path) -> Result<Vec<String>> {
    if evidence.image_only_pages.is_empty() {
        return Err("image-only page inventory is empty; complete CNA-167 first".into());
    }
    let mut seen = BTreeMap::new();
    let mut pending = Vec::new();
    for page in evidence.image_only_pages {
        let first_section = match page.book {
            Book::L21 => 1,
            Book::AL21 => 33,
            Book::SC21 => 59,
        };
        let key = format!("{:?}:{}", page.book, page.page);
        if !location(page.book, page.page, first_section) || seen.insert(key.clone(), ()).is_some()
        {
            return Err(format!("invalid/duplicate image-only page: {key}").into());
        }
        match page.status {
            PageStatus::Pending => pending.push(key),
            PageStatus::Transcribed => {
                let path = page
                    .transcript
                    .filter(|s| nonempty(s))
                    .ok_or_else(|| format!("missing transcript: {key}"))?;
                if !root.join(&path).is_file() {
                    return Err(format!("missing transcript file for {key}: {path}").into());
                }
            }
        }
    }
    Ok(pending)
}

fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).map_err(|e| {
        format!(
            "cannot read {}: {e}; provide the reviewed input or use --data-dir",
            path.display()
        )
    })?;
    serde_json::from_slice(&bytes).map_err(|e| format!("invalid {}: {e}", path.display()).into())
}

fn run(args: &[String]) -> Result<()> {
    if args == ["--help"] || args == ["-h"] {
        println!("{HELP}");
        return Ok(());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut data_dir = root.join("data/rules");
    let (mut ledger_check, mut pages) = (false, false);
    let (mut rebuild_index, mut check_index, mut custom_data_dir) = (false, false, false);
    let mut section = None;
    let mut required = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => ledger_check = true,
            "--check-pages" => pages = true,
            "--rebuild-index" => rebuild_index = true,
            "--check-index" => check_index = true,
            "--section" => {
                let n = args
                    .next()
                    .ok_or("--section needs a number 1–65")?
                    .parse::<u8>()?;
                if !(1..=65).contains(&n) {
                    return Err("--section must be 1–65".into());
                }
                section = Some(n);
            }
            "--require" => {
                for name in args
                    .next()
                    .ok_or("--require needs a comma-separated status list")?
                    .split(',')
                {
                    required.push(serde_json::from_value::<Status>(
                        serde_json::Value::String(name.into()),
                    )?);
                }
            }
            "--data-dir" => {
                data_dir = args.next().ok_or("--data-dir needs a path")?.into();
                custom_data_dir = true;
            }
            _ => return Err(format!("unknown option {arg}; see --help").into()),
        }
    }
    if !ledger_check && (section.is_some() || !required.is_empty()) {
        return Err("--section/--require need --check".into());
    }
    if rebuild_index && custom_data_dir {
        return Err("--rebuild-index writes repository outputs; omit --data-dir".into());
    }
    if !ledger_check && !pages && !rebuild_index && !check_index {
        return Err(
            "choose --check, --check-pages, --check-index or --rebuild-index; see --help".into(),
        );
    }
    if rebuild_index || check_index {
        index::run(&root, &data_dir, rebuild_index, check_index)?;
    }
    if ledger_check {
        let count = check(
            &read::<Vec<Case>>(&data_dir.join("case-index.json"))?,
            &read::<Vec<Row>>(&data_dir.join("ledger.json"))?,
            section,
            &required,
        )?;
        println!("ledger coverage passed: {count} cases");
    }
    if pages {
        let pending = check_pages(read(&data_dir.join("evidence-index.json"))?, &root)?;
        println!(
            "image-only page records checked; pending: {}",
            pending.join(", ")
        );
    }
    Ok(())
}

fn main() {
    if let Err(error) = run(&std::env::args().skip(1).collect::<Vec<_>>()) {
        eprintln!("rules-ledger: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn fixture() -> (Value, Vec<Value>) {
        let index = json!([
            {"case":"8.31","section":8,"book":"L21","page":12},
            {"case":"8.37","section":8,"book":"L21","page":13},
            {"case":"52.2","section":52,"book":"AL21","page":14}
        ]);
        let rows = [
            ("8.31", 8, "L21", 12),
            ("8.37", 8, "L21", 13),
            ("52.2", 52, "AL21", 14),
        ]
        .into_iter()
        .map(|(case, section, book, page)| {
            json!({
                "case":case,"section":section,"book":book,"page":page,
                "original_ref":"synthetic original reference","correction_refs":[],
                "july_text_ref":"synthetic July reference","vassal":"absent",
                "discrepancy":"synthetic fixture only","status":"verified","code_refs":[],"tests":[]
            })
        })
        .collect();
        (index, rows)
    }

    fn cli(index: &Value, values: &[Value], flags: &[&str]) -> Result<()> {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "cna-ledger-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("case-index.json"), index.to_string()).unwrap();
        fs::write(dir.join("ledger.json"), json!(values).to_string()).unwrap();
        let mut args = vec![
            "--data-dir".into(),
            dir.to_str().unwrap().into(),
            "--check".into(),
        ];
        args.extend(flags.iter().map(|s| s.to_string()));
        let result = run(&args);
        fs::remove_dir_all(dir).unwrap();
        result
    }

    #[test]
    fn completeness_and_required_status_are_not_vacuous() {
        let (index, mut values) = fixture();
        assert!(cli(&index, &values, &[]).is_ok());
        values[2]["status"] = json!("unread");
        assert!(cli(
            &index,
            &values,
            &["--section", "8", "--require", "verified"]
        )
        .is_ok());
        assert!(cli(&index, &values, &["--require", "verified"]).is_err());
        values[1]["status"] = json!("implemented");
        assert!(cli(
            &index,
            &values,
            &["--section", "8", "--require", "verified"]
        )
        .is_err());
        assert!(cli(
            &index,
            &values,
            &["--section", "8", "--require", "fixtures"]
        )
        .is_ok());
        values.remove(1);
        assert!(cli(&index, &values, &["--section", "8"]).is_err());
        assert!(cli(&index, &values, &["--section", "9"]).is_err());
        assert!(cli(&json!([]), &[], &[]).is_err());
    }

    #[test]
    fn false_coverage_unknown_cases_and_missing_evidence_fail() {
        let (index, values) = fixture();
        for (field, bad) in [
            ("case", json!("8.99")),
            ("page", json!(48)),
            ("vassal", json!("")),
            ("vassal_refs", json!(null)),
            ("vassal_refs", json!(["ambiguous with absent"])),
            ("status", json!("not_applicable")),
        ] {
            let mut changed = values.clone();
            changed[0][field] = bad;
            assert!(cli(&index, &changed, &[]).is_err());
        }
        let mut duplicate = values.clone();
        duplicate.push(values[0].clone());
        assert!(cli(&index, &duplicate, &[]).is_err());
        let mut exempt = values;
        exempt[0]["status"] = json!("not_applicable");
        exempt[0]["justification"] = json!("synthetic non-mechanical heading");
        assert!(cli(&index, &exempt, &["--require", "verified"]).is_err());
        assert!(cli(&index, &exempt, &["--require", "verified,not_applicable"]).is_ok());
    }
}
