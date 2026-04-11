#![allow(dead_code)]
mod charstate;
mod easing;
mod effects;
mod engine;
mod gradient;
#[cfg(test)]
mod tests;

use clap::Parser;
use engine::{run_animation, Grid};
use rand::seq::SliceRandom;
use std::io::{self, Read};

const ALL_EFFECTS: &[&str] = &[
    "beams",
    "binarypath",
    "blackhole",
    "bouncyballs",
    "bubbles",
    "burn",
    "colorshift",
    "crumble",
    "decrypt",
    "errorcorrect",
    "expand",
    "fireworks",
    "highlight",
    "laseretch",
    "matrix",
    "middleout",
    "orbittingvolley",
    "overflow",
    "pour",
    "print",
    "rain",
    "randomsequence",
    "rings",
    "scattered",
    "slice",
    "slide",
    "smoke",
    "spotlights",
    "spray",
    "swarm",
    "sweep",
    "synthgrid",
    "thunderstorm",
    "unstable",
    "vhstape",
    "waves",
    "wipe",
];

#[derive(Parser)]
#[command(name = "rtte", about = "A fast terminal text effects engine")]
struct Cli {
    /// Effect to apply
    #[arg(value_name = "EFFECT")]
    effect: Option<String>,

    /// Randomly select an effect
    #[arg(short = 'R', long = "random-effect")]
    random_effect: bool,

    /// Target frame rate
    #[arg(long = "frame-rate", default_value = "60")]
    frame_rate: u32,

    /// Input file (reads stdin if not provided)
    #[arg(short = 'i', long = "input-file")]
    input_file: Option<String>,

    /// Effects to include when randomly selecting
    #[arg(long = "include-effects", num_args = 1..)]
    include_effects: Option<Vec<String>>,

    /// Effects to exclude when randomly selecting
    #[arg(long = "exclude-effects", num_args = 1..)]
    exclude_effects: Option<Vec<String>>,

    /// Do not restore cursor position
    #[arg(long = "no-restore-cursor")]
    _no_restore_cursor: bool,

    /// Show version
    #[arg(short = 'v', long = "version")]
    version: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.version {
        println!("rtte 0.1.0");
        return;
    }

    // Read input
    let input = if let Some(path) = &cli.input_file {
        std::fs::read_to_string(path).expect("Failed to read input file")
    } else {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .expect("Failed to read stdin");
        buf
    };

    if input.trim().is_empty() {
        return;
    }

    // Select effect
    let mut rng = rand::thread_rng();
    let effect_name = if cli.random_effect {
        let mut pool: Vec<&str> = if let Some(ref inc) = cli.include_effects {
            ALL_EFFECTS
                .iter()
                .filter(|e| inc.iter().any(|i| i == **e))
                .copied()
                .collect()
        } else if let Some(ref exc) = cli.exclude_effects {
            ALL_EFFECTS
                .iter()
                .filter(|e| !exc.iter().any(|x| x == **e))
                .copied()
                .collect()
        } else {
            ALL_EFFECTS.to_vec()
        };
        if pool.is_empty() {
            pool = ALL_EFFECTS.to_vec();
        }
        pool.choose(&mut rng).unwrap().to_string()
    } else if let Some(ref e) = cli.effect {
        e.clone()
    } else {
        eprintln!("Error: specify an effect or use --random-effect");
        std::process::exit(1);
    };

    let mut grid = Grid::from_input(&input);

    // Create the selected stateful effect
    enum Effect {
        Beams(effects::BeamsEffect),
        BinaryPath(effects::BinaryPathEffect),
        Blackhole(effects::BlackholeEffect),
        BouncyBalls(effects::BouncyBallsEffect),
        Bubbles(effects::BubblesEffect),
        Burn(effects::BurnEffect),
        ColorShift(effects::ColorShiftEffect),
        Crumble(effects::CrumbleEffect),
        Decrypt(effects::DecryptEffect),
        ErrorCorrect(effects::ErrorCorrectEffect),
        Expand(effects::ExpandEffect),
        Fireworks(effects::FireworksEffect),
        Highlight(effects::HighlightEffect),
        LaserEtch(effects::LaserEtchEffect),
        Matrix(effects::MatrixEffect),
        MiddleOut(effects::MiddleOutEffect),
        OrbittingVolley(effects::OrbittingVolleyEffect),
        Overflow(effects::OverflowEffect),
        Pour(effects::PourEffect),
        Print(effects::PrintEffect),
        Rain(effects::RainEffect),
        RandomSequence(effects::RandomSequenceEffect),
        Rings(effects::RingsEffect),
        Scattered(effects::ScatteredEffect),
        Slice(effects::SliceEffect),
        Slide(effects::SlideEffect),
        Smoke(effects::SmokeEffect),
        Spotlights(effects::SpotlightsEffect),
        Spray(effects::SprayEffect),
        Swarm(effects::SwarmEffect),
        Sweep(effects::SweepEffect),
        SynthGrid(effects::SynthGridEffect),
        Thunderstorm(effects::ThunderstormEffect),
        Unstable(effects::UnstableEffect),
        VHSTape(effects::VHSTapeEffect),
        Waves(effects::WavesEffect),
        Wipe(effects::WipeEffect),
    }

    let mut effect = match effect_name.as_str() {
        "beams" => Effect::Beams(effects::BeamsEffect::new(&grid)),
        "binarypath" => Effect::BinaryPath(effects::BinaryPathEffect::new(&grid)),
        "blackhole" => Effect::Blackhole(effects::BlackholeEffect::new(&grid)),
        "bouncyballs" => Effect::BouncyBalls(effects::BouncyBallsEffect::new(&grid)),
        "bubbles" => Effect::Bubbles(effects::BubblesEffect::new(&grid)),
        "burn" => Effect::Burn(effects::BurnEffect::new(&grid)),
        "colorshift" => Effect::ColorShift(effects::ColorShiftEffect::new(&grid)),
        "crumble" => Effect::Crumble(effects::CrumbleEffect::new(&grid)),
        "decrypt" => Effect::Decrypt(effects::DecryptEffect::new(&grid)),
        "errorcorrect" => Effect::ErrorCorrect(effects::ErrorCorrectEffect::new(&grid)),
        "expand" => Effect::Expand(effects::ExpandEffect::new(&grid)),
        "fireworks" => Effect::Fireworks(effects::FireworksEffect::new(&grid)),
        "highlight" => Effect::Highlight(effects::HighlightEffect::new(&grid)),
        "laseretch" => Effect::LaserEtch(effects::LaserEtchEffect::new(&grid)),
        "matrix" => Effect::Matrix(effects::MatrixEffect::new(&grid)),
        "middleout" => Effect::MiddleOut(effects::MiddleOutEffect::new(&grid)),
        "orbittingvolley" => Effect::OrbittingVolley(effects::OrbittingVolleyEffect::new(&grid)),
        "overflow" => Effect::Overflow(effects::OverflowEffect::new(&grid)),
        "pour" => Effect::Pour(effects::PourEffect::new(&grid)),
        "print" => Effect::Print(effects::PrintEffect::new(&grid)),
        "rain" => Effect::Rain(effects::RainEffect::new(&grid)),
        "randomsequence" => Effect::RandomSequence(effects::RandomSequenceEffect::new(&grid)),
        "rings" => Effect::Rings(effects::RingsEffect::new(&grid)),
        "scattered" => Effect::Scattered(effects::ScatteredEffect::new(&grid)),
        "slice" => Effect::Slice(effects::SliceEffect::new(&grid)),
        "slide" => Effect::Slide(effects::SlideEffect::new(&grid)),
        "smoke" => Effect::Smoke(effects::SmokeEffect::new(&grid)),
        "spotlights" => Effect::Spotlights(effects::SpotlightsEffect::new(&grid)),
        "spray" => Effect::Spray(effects::SprayEffect::new(&grid)),
        "swarm" => Effect::Swarm(effects::SwarmEffect::new(&grid)),
        "sweep" => Effect::Sweep(effects::SweepEffect::new(&grid)),
        "synthgrid" => Effect::SynthGrid(effects::SynthGridEffect::new(&grid)),
        "thunderstorm" => Effect::Thunderstorm(effects::ThunderstormEffect::new(&grid)),
        "unstable" => Effect::Unstable(effects::UnstableEffect::new(&grid)),
        "vhstape" => Effect::VHSTape(effects::VHSTapeEffect::new(&grid)),
        "waves" => Effect::Waves(effects::WavesEffect::new(&grid)),
        "wipe" => Effect::Wipe(effects::WipeEffect::new(&grid)),
        _ => {
            eprintln!("Unknown effect: {}", effect_name);
            std::process::exit(1);
        }
    };

    run_animation(
        &mut grid,
        cli.frame_rate,
        |grid, _frame| match &mut effect {
            Effect::Beams(e) => e.tick(grid),
            Effect::BinaryPath(e) => e.tick(grid),
            Effect::Blackhole(e) => e.tick(grid),
            Effect::BouncyBalls(e) => e.tick(grid),
            Effect::Bubbles(e) => e.tick(grid),
            Effect::Burn(e) => e.tick(grid),
            Effect::ColorShift(e) => e.tick(grid),
            Effect::Crumble(e) => e.tick(grid),
            Effect::Decrypt(e) => e.tick(grid),
            Effect::ErrorCorrect(e) => e.tick(grid),
            Effect::Expand(e) => e.tick(grid),
            Effect::Fireworks(e) => e.tick(grid),
            Effect::Highlight(e) => e.tick(grid),
            Effect::LaserEtch(e) => e.tick(grid),
            Effect::Matrix(e) => e.tick(grid),
            Effect::MiddleOut(e) => e.tick(grid),
            Effect::OrbittingVolley(e) => e.tick(grid),
            Effect::Overflow(e) => e.tick(grid),
            Effect::Pour(e) => e.tick(grid),
            Effect::Print(e) => e.tick(grid),
            Effect::Rain(e) => e.tick(grid),
            Effect::RandomSequence(e) => e.tick(grid),
            Effect::Rings(e) => e.tick(grid),
            Effect::Scattered(e) => e.tick(grid),
            Effect::Slice(e) => e.tick(grid),
            Effect::Slide(e) => e.tick(grid),
            Effect::Smoke(e) => e.tick(grid),
            Effect::Spotlights(e) => e.tick(grid),
            Effect::Spray(e) => e.tick(grid),
            Effect::Swarm(e) => e.tick(grid),
            Effect::Sweep(e) => e.tick(grid),
            Effect::SynthGrid(e) => e.tick(grid),
            Effect::Thunderstorm(e) => e.tick(grid),
            Effect::Unstable(e) => e.tick(grid),
            Effect::VHSTape(e) => e.tick(grid),
            Effect::Waves(e) => e.tick(grid),
            Effect::Wipe(e) => e.tick(grid),
        },
    );
}
