//! Kernel argv construction.

use std::process::Command;

/// Build the argv to launch the kernel. If the user provided custom argv
/// (via the trailing `--`), use it verbatim. Otherwise default to ipython
/// with `python3` resolved to an absolute path — kallichore rejects
/// relative kernel paths.
pub fn build_argv(custom: &[String]) -> Vec<String> {
    if !custom.is_empty() {
        return custom.to_vec();
    }
    let python = which_python().unwrap_or_else(|| "python3".into());
    vec![
        python,
        "-m".into(),
        "ipykernel_launcher".into(),
        "-f".into(),
        "{connection_file}".into(),
    ]
}

fn which_python() -> Option<String> {
    for name in ["python3", "python"] {
        if let Ok(out) = Command::new("which").arg(name).output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_custom_when_provided() {
        let custom = vec!["foo".to_string(), "bar".to_string()];
        assert_eq!(build_argv(&custom), custom);
    }

    #[test]
    fn default_includes_connection_file_placeholder() {
        let argv = build_argv(&[]);
        assert!(argv.iter().any(|a| a == "{connection_file}"));
        assert!(argv.iter().any(|a| a.contains("ipykernel")));
    }
}
