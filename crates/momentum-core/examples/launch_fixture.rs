// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Dan Hart
//! Seed an isolated, synthetic native-launch benchmark fixture.
use momentum_core::{Engine, View};
use std::{env, fs, io, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    let [path, count] = args.as_slice() else {
        return Err("usage: launch_fixture NEW_DIRECTORY 10|1000".into());
    };
    let count = match count.to_str() {
        Some("10") => 10,
        Some("1000") => 1000,
        _ => return Err("fixture size must be 10 or 1000".into()),
    };
    let directory = PathBuf::from(path);
    let path = directory.to_str().ok_or("fixture paths must be UTF-8")?;
    fs::create_dir(&directory)?;
    let engine = Engine::open_checked(path.into())?;
    let source = (1..=count)
        .map(|index| format!("Momentum launch fixture {index:04}"))
        .collect::<Vec<_>>()
        .join("\n");
    if !engine.add_from_text(source, View::Today).changed || engine.all_tasks().len() != count {
        return Err(io::Error::other("could not create the complete benchmark fixture").into());
    }
    Ok(())
}
