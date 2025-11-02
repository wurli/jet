/*
 * startup.rs
 *
 * Copyright (C) 2025 Jacob Scott. All rights reserved.
 *
 */

use std::{fs::metadata, path::PathBuf};

use jet::api;

fn path_exists(path: &PathBuf) -> bool {
    metadata(path).is_ok()
}

#[test]
fn can_start_ark() {
    let jupyter_path = if let Some(path) = std::env::var_os("JUPYTER_PATH") {
        PathBuf::from(path)
    } else if let Some(path) = std::env::var_os("PWD") {
        PathBuf::from(path)
    } else {
        eprintln!("No JUPYTER_PATH or PWD set, cannot run test");
        return;
    };

    let ark_path = jupyter_path.join("kernels/ark/kernel.json");
    eprintln!("Looking for ark kernel at: {}", ark_path.display());

    if !path_exists(&ark_path) {
        eprintln!("Looking for ark kernel at: {}", ark_path.display());
        return;
    }

    let (_id, _kernel) = api::start_kernel(ark_path).expect("Could not start ark kernel");
}

// #[test]
// fn can_start_ipykernel() {
//     let ark_path = PathBuf::from("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json");
//     if !path_exists(&ark_path) {
//         return;
//     }
//
//     let (_id, _kernel) = api::start_kernel(ark_path).expect("Could not start ipython kernel");
// }
//
//
// #[test]
// fn can_start_evecxr() {
//     let ark_path = PathBuf::from("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/rust/kernel.json");
//     if !path_exists(&ark_path) {
//         return;
//     }
//
//     let (_id, _kernel) = api::start_kernel(ark_path).expect("Could not start evecxr kernel");
// }
