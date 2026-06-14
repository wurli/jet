//! Kernel argv construction.

const CONNECTION_FILE_PLACEHOLDER: &str = "{connection_file}";

/// Pass the user's argv through, appending `-f {connection_file}` if no
/// argument already contains the placeholder. Kallichore substitutes the
/// placeholder with the path to the generated connection file at launch.
pub fn build_argv(custom: &[String]) -> Vec<String> {
    let mut argv = custom.to_vec();
    if !argv.iter().any(|a| a.contains(CONNECTION_FILE_PLACEHOLDER)) {
        argv.push("-f".into());
        argv.push(CONNECTION_FILE_PLACEHOLDER.into());
    }
    argv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_custom_through_when_placeholder_present() {
        let custom = vec!["ark".into(), "--connection_file".into(), "{connection_file}".into()];
        assert_eq!(build_argv(&custom), custom);
    }

    #[test]
    fn appends_connection_file_when_missing() {
        let custom = vec!["python3".into(), "-m".into(), "ipykernel_launcher".into()];
        let argv = build_argv(&custom);
        assert_eq!(argv.last().unwrap(), "{connection_file}");
        assert_eq!(argv[argv.len() - 2], "-f");
    }

    #[test]
    fn detects_placeholder_inside_a_larger_arg() {
        let custom = vec!["ark".into(), "--conn={connection_file}".into()];
        assert_eq!(build_argv(&custom), custom);
    }
}
