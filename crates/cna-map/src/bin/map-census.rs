//! Source inventory only: VASSAL grids are calibration evidence, not terrain authority.
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const SOURCE: &str = "docs/CNAv2.1.0/buildFile.xml";
const OUTPUT: &str = "public/data/map/sources.json";
const MAP: &str = "VASSAL.build.module.Map";
const BOARD: &str = "VASSAL.build.module.map.boardPicker.Board";
const ZONE: &str = "VASSAL.build.module.map.boardPicker.board.mapgrid.Zone";
const HEX: &str = "VASSAL.build.module.map.boardPicker.board.HexGrid";
const SQUARE: &str = "VASSAL.build.module.map.boardPicker.board.SquareGrid";
const HEX_NUMBERING: &str = "VASSAL.build.module.map.boardPicker.board.mapgrid.HexGridNumbering";
const SQUARE_NUMBERING: &str =
    "VASSAL.build.module.map.boardPicker.board.mapgrid.SquareGridNumbering";

#[derive(Serialize)]
struct Census {
    schema_version: u32,
    source: Source,
    map_name: String,
    board_name: String,
    image: Image,
    hex_cell_estimate_method: &'static str,
    zones: Vec<Zone>,
}

#[derive(Serialize)]
struct Source {
    path: String,
    sha256: String,
}

#[derive(Serialize)]
struct Image {
    file_name: String,
    #[serde(flatten)]
    source: Source,
    width: u32,
    height: u32,
}

#[derive(Serialize)]
struct Zone {
    name: String,
    kind: &'static str,
    polygon: Vec<[i32; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hex_cell_estimate: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    grid: Option<Grid>,
}

#[derive(Serialize)]
struct Grid {
    kind: &'static str,
    attributes: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    numbering: Option<BTreeMap<String, String>>,
}

fn attribute<'a>(node: Node<'a, '_>, key: &str) -> Result<&'a str> {
    node.attribute(key)
        .ok_or_else(|| format!("{} missing {key}", node.tag_name().name()).into())
}

fn attributes(node: Node<'_, '_>) -> BTreeMap<String, String> {
    node.attributes()
        .map(|a| (a.name().to_owned(), a.value().to_owned()))
        .collect()
}

fn only<'a, 'input>(
    mut nodes: impl Iterator<Item = Node<'a, 'input>>,
    label: &str,
) -> Result<Node<'a, 'input>> {
    let first = nodes.next().ok_or_else(|| format!("missing {label}"))?;
    if nodes.next().is_some() {
        return Err(format!("expected exactly one {label}").into());
    }
    Ok(first)
}

fn non_hex_kind(name: &str) -> Result<&'static str> {
    match name {
        "Sicily Holding Box"
        | "Italy Holding Box"
        | "Benghazi Holding Box"
        | "Derna Holding Box"
        | "Crete Holding Box"
        | "Bardia Holding Box"
        | "Tobruk Holding Box"
        | "Alexandria Holding Box Hex 3613"
        | "Alexandria  Holding Box Hex 3714"
        | "Tunis"
        | "Gabes"
        | "Tripoli"
        | "Tripolitania"
        | "German Shipping" => Ok("off_map_box"),
        "Tunis-Gabes" | "Gabes-Tripoli" | "Tripoli-Tripolitania" | "Tripolitania-Nofilia" => {
            Ok("track")
        }
        "Weather Track"
        | "Map A Weather"
        | "Map B Weather"
        | "Map C Weather"
        | "Map D Weather"
        | "Map E Weather"
        | "Map D Operation Stage"
        | "Map E Operation Stage"
        | "Game Track" => Ok("ui_track"),
        _ => Err(
            format!("unclassified zone {name:?}; review its source before adding a kind").into(),
        ),
    }
}

fn zone(node: Node<'_, '_>) -> Result<Zone> {
    let name = attribute(node, "name")?.to_owned();
    let polygon = attribute(node, "path")?
        .split(';')
        .map(|pair| -> Result<[i32; 2]> {
            let (x, y) = pair.split_once(',').ok_or("invalid polygon coordinate")?;
            Ok([x.parse()?, y.parse()?])
        })
        .collect::<Result<Vec<_>>>()?;
    if polygon.len() < 3 {
        return Err(format!("zone {name:?} needs at least three polygon points").into());
    }
    let children: Vec<_> = node.children().filter(Node::is_element).collect();
    let grid = if children.is_empty() {
        None
    } else {
        let g = only(children.into_iter(), "zone grid")?;
        let (kind, numbering_tag) = match g.tag_name().name() {
            HEX => ("hex", HEX_NUMBERING),
            SQUARE => ("square", SQUARE_NUMBERING),
            other => return Err(format!("unsupported grid {other} in {name}").into()),
        };
        let numbering_nodes: Vec<_> = g.children().filter(Node::is_element).collect();
        let numbering = if numbering_nodes.is_empty() && kind == "square" {
            None
        } else {
            let n = only(numbering_nodes.into_iter(), "grid numbering")?;
            if !n.has_tag_name(numbering_tag) {
                return Err(format!("unexpected numbering in {name}").into());
            }
            Some(attributes(n))
        };
        Some(Grid {
            kind,
            attributes: attributes(g),
            numbering,
        })
    };
    let is_hex = grid.as_ref().is_some_and(|g| g.kind == "hex");
    let kind = if is_hex {
        "hex_section"
    } else {
        non_hex_kind(&name)?
    };
    let hex_cell_estimate = if let Some(g) = grid.as_ref().filter(|_| is_hex) {
        let dx: f64 = g.attributes.get("dx").ok_or("missing dx")?.parse()?;
        let dy: f64 = g.attributes.get("dy").ok_or("missing dy")?.parse()?;
        if !dx.is_finite() || !dy.is_finite() || dx <= 0.0 || dy <= 0.0 {
            return Err(format!("invalid hex spacing in {name}").into());
        }
        // Shoelace area / lattice-cell area. This is not a clipped-center count.
        let twice_area: f64 = polygon
            .iter()
            .zip(polygon.iter().cycle().skip(1))
            .map(|(a, b)| f64::from(a[0]) * f64::from(b[1]) - f64::from(b[0]) * f64::from(a[1]))
            .sum();
        Some((twice_area.abs() / (2.0 * dx * dy)).round() as u64)
    } else {
        None
    };
    Ok(Zone {
        name,
        kind,
        polygon,
        hex_cell_estimate,
        grid,
    })
}

fn census(root: &Path) -> Result<Census> {
    let xml = fs::read_to_string(root.join(SOURCE))?;
    let document = Document::parse(&xml)?;
    let map = only(
        document
            .descendants()
            .filter(|n| n.has_tag_name(MAP) && n.attribute("mapName") == Some("Main Map")),
        "Main Map",
    )?;
    let board = only(
        map.descendants().filter(|n| n.has_tag_name(BOARD)),
        "Main Map board",
    )?;
    let file_name = attribute(board, "image")?.to_owned();
    // The source image must remain a local file within the supplied image directory.
    if Path::new(&file_name).components().count() != 1
        || Path::new(&file_name).file_name().is_none()
    {
        return Err("board image must be a file name, not a path".into());
    }
    let image_path = format!("docs/CNAv2.1.0/images/{file_name}");
    let mut image = fs::File::open(root.join(&image_path))?;
    let mut header = [0; 24];
    image.read_exact(&mut header)?;
    if &header[..16] != b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR" {
        return Err("board image is not a PNG with an IHDR header".into());
    }
    let width = u32::from_be_bytes(header[16..20].try_into()?);
    let height = u32::from_be_bytes(header[20..24].try_into()?);
    let mut digest = Sha256::new();
    digest.update(header);
    let mut buffer = [0; 65536];
    loop {
        let n = image.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    let zones = map
        .descendants()
        .filter(|n| n.has_tag_name(ZONE))
        .map(zone)
        .collect::<Result<Vec<_>>>()?;
    let names: BTreeSet<_> = zones.iter().map(|z| &z.name).collect();
    if names.len() != zones.len() {
        return Err("duplicate zone names in Main Map".into());
    }
    Ok(Census {
        schema_version: 1,
        source: Source { path: SOURCE.into(), sha256: format!("{:x}", Sha256::digest(xml.as_bytes())) },
        map_name: attribute(map, "mapName")?.into(),
        board_name: attribute(board, "name")?.into(),
        image: Image {
            file_name,
            source: Source { path: image_path, sha256: format!("{:x}", digest.finalize()) },
            width,
            height,
        },
        hex_cell_estimate_method: "round(abs(polygon shoelace area) / (dx * dy)); approximate, not playable-cell or clipped-center count",
        zones,
    })
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum DistributionStatus {
    Pending,
    LocalOnly,
    Granted,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DistributionPolicy {
    status: DistributionStatus,
    grant_reference: Option<String>,
}

fn contains_payload(path: &Path) -> Result<bool> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    // A link is payload too: do not follow it outside the bundle or overlook it.
    if !metadata.is_dir() {
        return Ok(true);
    }
    for entry in fs::read_dir(path)? {
        if contains_payload(&entry?.path())? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn release_check(root: &Path, args: &[String]) -> Result<()> {
    let mut bundle = root.join("dist");
    let mut local_only = false;
    let mut options = args.iter();
    while let Some(arg) = options.next() {
        match arg.as_str() {
            "--bundle-dir" => {
                let path = options
                    .next()
                    .filter(|v| !v.starts_with('-'))
                    .ok_or("--bundle-dir needs a directory; see map-census --help")?;
                bundle = root.join(path);
            }
            "--local-only" => local_only = true,
            _ => {
                return Err(
                    format!("unexpected release-check option {arg}; see map-census --help").into(),
                )
            }
        }
    }
    let policy: DistributionPolicy =
        serde_json::from_slice(&fs::read(root.join("docs/map/distribution.json"))?)?;
    if policy.status == DistributionStatus::Granted
        && policy
            .grant_reference
            .as_ref()
            .is_none_or(|r| r.trim().is_empty())
    {
        return Err("granted map distribution requires a nonempty grant_reference".into());
    }
    if contains_payload(&bundle.join("tiles"))?
        && policy.status != DistributionStatus::Granted
        && !local_only
    {
        return Err(format!("source-derived tiles in {}/tiles are not cleared for distribution; remove them or record Kevin's grant decision in docs/map/distribution.json. For private research only, use --release-check --local-only", bundle.display()).into());
    }
    if local_only {
        println!("local-only map check passed; this is NOT permission to publish the bundle");
    } else {
        println!("map distribution check passed: {}", bundle.display());
    }
    Ok(())
}

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--help"] || args == ["-h"] {
        println!("map-census — inventory the supplied VASSAL map sources\n\nUsage: map-census [--check | --write | --release-check [--bundle-dir DIR] [--local-only]]\n\n  --check          Compare source-derived JSON with public/data/map/sources.json; never write\n  --write          Regenerate public/data/map/sources.json deterministically\n  --release-check  Reject uncleared tiles under DIR/tiles using docs/map/distribution.json\n  --bundle-dir DIR Bundle to inspect (default: dist); relative to repository root\n  --local-only     Private research override; NOT permission to publish\n  --help           Show this help\n\nNo flag defaults to --check. Paths are relative to the build-time repository root.\n\nExamples:\n  cargo run -p cna-map --bin map-census -- --write\n  cargo run -p cna-map --bin map-census -- --check\n  cargo run -p cna-map --bin map-census -- --release-check\n  cargo run -p cna-map --bin map-census -- --release-check --bundle-dir public --local-only");
        return Ok(());
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    if args.first().is_some_and(|a| a == "--release-check") {
        return release_check(&root, &args[1..]);
    }
    if !args.is_empty() && args != ["--check"] && args != ["--write"] {
        return Err(
            "expected --check, --write or --release-check; run map-census --help for examples"
                .into(),
        );
    }
    let census = census(&root)?;
    let text = serde_json::to_string_pretty(&census)? + "\n";
    let output = root.join(OUTPUT);
    if args == ["--write"] {
        fs::create_dir_all(output.parent().ok_or("output has no parent")?)?;
        fs::write(output, &text)?;
    } else {
        let existing = fs::read_to_string(output)
            .map_err(|e| format!("{OUTPUT}: {e}; regenerate with map-census --write"))?;
        if existing != text {
            return Err(format!("{OUTPUT} differs from source census; inspect source changes, then run map-census --write").into());
        }
    }
    let grids = census
        .zones
        .iter()
        .filter(|z| z.kind == "hex_section")
        .count();
    println!(
        "{}: {grids} hex grids, {} zones, image {}x{} SHA256 {}",
        if args == ["--write"] {
            "wrote census"
        } else {
            "census matches"
        },
        census.zones.len(),
        census.image.width,
        census.image.height,
        census.image.source.sha256
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("map-census: {error}");
            ExitCode::FAILURE
        }
    }
}
