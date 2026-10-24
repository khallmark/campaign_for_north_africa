//! Deterministic, local-research PNG pyramid. This grants no distribution rights.
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{imageops, ImageEncoder, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fs, path::Path, path::PathBuf, process::ExitCode};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const TILE_SIZE: u32 = 512;
// Bump when changing resizing, color conversion or encoder settings.
const GENERATOR: &str = "cna-map-tiles-v1/image-0.25.10/rgba8-lanczos3/png-best-adaptive";

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    generator: String,
    source_sha256: String,
    board_width: u32,
    board_height: u32,
    tile_size: u32,
    levels: Vec<Level>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Level {
    z: u32,
    width: u32,
    height: u32,
    downsample: u32,
    columns: u32,
    rows: u32,
    tiles: Vec<Tile>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Tile {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    sha256: String,
    bytes: u64,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn plan(width: u32, height: u32, source_sha256: String) -> Manifest {
    let mut top = 0;
    while width.max(height).div_ceil(1 << top) > TILE_SIZE {
        top += 1;
    }
    let levels = (0..=top)
        .map(|z| {
            let downsample = 1 << (top - z);
            let (width, height) = (width.div_ceil(downsample), height.div_ceil(downsample));
            let (columns, rows) = (width.div_ceil(TILE_SIZE), height.div_ceil(TILE_SIZE));
            let tiles = (0..rows)
                .flat_map(|y| {
                    (0..columns).map(move |x| Tile {
                        x,
                        y,
                        width: TILE_SIZE.min(width - x * TILE_SIZE),
                        height: TILE_SIZE.min(height - y * TILE_SIZE),
                        sha256: String::new(),
                        bytes: 0,
                    })
                })
                .collect();
            Level {
                z,
                width,
                height,
                downsample,
                columns,
                rows,
                tiles,
            }
        })
        .collect();
    Manifest {
        generator: GENERATOR.into(),
        source_sha256,
        board_width: width,
        board_height: height,
        tile_size: TILE_SIZE,
        levels,
    }
}

fn tile_path(output: &Path, z: u32, tile: &Tile) -> PathBuf {
    output
        .join(z.to_string())
        .join(format!("{}_{}.png", tile.x, tile.y))
}

fn unchanged(expected: &Manifest, existing: &Manifest, output: &Path) -> bool {
    // Compare geometry/settings as well as the source hash. Derive paths from our
    // plan, never from arbitrary paths in a pre-existing manifest.
    let mut shape = plan(
        existing.board_width,
        existing.board_height,
        existing.source_sha256.clone(),
    );
    shape.generator = existing.generator.clone();
    shape.tile_size = existing.tile_size;
    if &shape != expected || existing.levels.len() != shape.levels.len() {
        return false;
    }
    for (level, saved) in shape.levels.iter_mut().zip(&existing.levels) {
        if level.tiles.len() != saved.tiles.len() {
            return false;
        }
        for (tile, saved_tile) in level.tiles.iter_mut().zip(&saved.tiles) {
            let Ok(bytes) = fs::read(tile_path(output, level.z, tile)) else {
                return false;
            };
            tile.sha256 = hash(&bytes);
            tile.bytes = bytes.len() as u64;
            if tile != saved_tile {
                return false;
            }
        }
    }
    shape == *existing
}

fn generate(source: &Path, output: &Path) -> Result<()> {
    let bytes = fs::read(source).map_err(|e| format!("{}: {e}", source.display()))?;
    let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?.to_rgba8();
    let mut manifest = plan(image.width(), image.height(), hash(&bytes));
    let manifest_path = output.join("manifest.json");
    if output.exists() && fs::read_dir(output)?.next().is_some() {
        let saved: Manifest =
            serde_json::from_slice(&fs::read(&manifest_path).map_err(|_| {
                "nonempty output has no manifest; choose an empty --output directory"
            })?)?;
        if saved.board_width != image.width() || saved.board_height != image.height() {
            return Err(
                "output belongs to a different board size; choose an empty --output directory"
                    .into(),
            );
        }
        if unchanged(&manifest, &saved, output) {
            println!(
                "unchanged: {} SHA256 {}",
                manifest_path.display(),
                hash(&fs::read(&manifest_path)?)
            );
            return Ok(());
        }
    }
    fs::create_dir_all(output)?;
    for level in &mut manifest.levels {
        let resized: RgbaImage;
        let pixels = if level.downsample == 1 {
            &image
        } else {
            resized = imageops::resize(
                &image,
                level.width,
                level.height,
                imageops::FilterType::Lanczos3,
            );
            &resized
        };
        fs::create_dir_all(output.join(level.z.to_string()))?;
        for tile in &mut level.tiles {
            let crop = imageops::crop_imm(
                pixels,
                tile.x * TILE_SIZE,
                tile.y * TILE_SIZE,
                tile.width,
                tile.height,
            )
            .to_image();
            let mut encoded = Vec::new();
            PngEncoder::new_with_quality(&mut encoded, CompressionType::Best, FilterType::Adaptive)
                .write_image(
                    crop.as_raw(),
                    tile.width,
                    tile.height,
                    image::ExtendedColorType::Rgba8,
                )?;
            tile.sha256 = hash(&encoded);
            tile.bytes = encoded.len() as u64;
            fs::write(tile_path(output, level.z, tile), encoded)?;
        }
    }
    // Publish the manifest last. An interrupted regeneration cannot pass the
    // prior manifest's hashes; a first run without a manifest needs fresh output.
    let encoded = serde_json::to_string_pretty(&manifest)? + "\n";
    fs::write(&manifest_path, &encoded)?;
    println!(
        "generated: {} levels, {}x{}, {} SHA256 {}",
        manifest.levels.len(),
        image.width(),
        image.height(),
        manifest_path.display(),
        hash(encoded.as_bytes())
    );
    eprintln!("Local research output only; release distribution guard remains mandatory.");
    Ok(())
}

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args == ["--help"] || args == ["-h"] {
        println!("cna-map-tiles — deterministic local-research PNG pyramid\n\nUsage: cna-map-tiles [--source PNG] [--output DIR]\n\nDefaults: supplied CNA map; public/tiles/map (repository-relative).\nExplicit paths are relative to the current directory. Tiles are 512px; edge\ntiles are cropped, not stretched. z=0 fits one tile; highest z is full size.\nUnchanged source/settings and all tile checksums skip writes. Different board\nsizes require an empty output directory. No distribution permission is implied.\n\nExamples:\n  npm run tiles\n  npm run tiles -- --output .amp/in/local-tiles\n  cna-map-tiles --source fixture.png --output /tmp/fixture-tiles");
        return Ok(());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut source = root.join("docs/CNAv2.1.0/images/CNA Map Vassal Mitch Guthrie 2021.png");
    let mut output = root.join("public/tiles/map");
    let mut options = args.iter();
    while let Some(option) = options.next() {
        let destination = if option == "--source" {
            &mut source
        } else if option == "--output" {
            &mut output
        } else {
            return Err(format!(
                "unexpected option {}; see --help for examples",
                option.to_string_lossy()
            )
            .into());
        };
        *destination = PathBuf::from(
            options
                .next()
                .filter(|p| !p.to_string_lossy().starts_with('-'))
                .ok_or("option requires a path; see --help for examples")?,
        );
    }
    generate(&source, &output)
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("cna-map-tiles: {error}");
            ExitCode::FAILURE
        }
    }
}
