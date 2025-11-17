use jet::kernel::kernel_spec::KernelSpec;

#[test]
fn jet_can_discover_kernel_specs() {
    let kernels = KernelSpec::find_valid();

    let expected_short_names = vec![String::from("ark"), String::from("python3")];

    let actual_short_names = kernels
        .keys()
        .map(|key| {
            key.parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect::<Vec<String>>();

    let actual_full_paths = kernels
        .keys()
        .map(|key| key.to_string_lossy().to_string())
        .collect::<Vec<String>>()
        .join(",\n");

    for kernel_name in expected_short_names.iter() {
        assert!(
            actual_short_names.contains(kernel_name),
            "Kernel '{kernel_name}' not detected. Detected kernels:\n{actual_full_paths}"
        )
    }
}
