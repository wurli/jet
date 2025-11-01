use std::{fs::metadata, path::PathBuf};

use carpo::api;

fn path_exists(path: &PathBuf) -> bool {
    metadata(path).is_ok()
}

#[test]
fn can_start_ark() {
    let ark_path = PathBuf::from("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark/kernel.json");
    if !path_exists(&ark_path) {
        return;
    }

    let (_id, _kernel) = api::start_kernel(ark_path).expect("Could not start ark kernel");
}

#[test]
fn can_start_ipykernel() {
    let ark_path = PathBuf::from("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json");
    if !path_exists(&ark_path) {
        return;
    }

    let (_id, _kernel) = api::start_kernel(ark_path).expect("Could not start ipython kernel");
}


#[test]
fn can_start_evecxr() {
    let ark_path = PathBuf::from("/Users/JACOB.SCOTT1/Library/Jupyter/kernels/rust/kernel.json");
    if !path_exists(&ark_path) {
        return;
    }

    let (_id, _kernel) = api::start_kernel(ark_path).expect("Could not start evecxr kernel");
}
