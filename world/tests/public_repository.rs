use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("world crate must be inside repository")
        .to_path_buf()
}

#[test]
fn readme_explains_the_complete_public_project() {
    let readme = fs::read_to_string(repository_root().join("README.md")).expect("read README");

    for required_section in [
        "## Что такое MakiseWE",
        "## Архитектурные принципы",
        "## Текущий статус",
        "## Что уже работает",
        "## Целевой охват V1",
        "## Структура репозитория",
        "## Быстрый старт",
        "## Проверка",
        "## Документация",
        "## Участие в разработке",
        "## Безопасность",
        "## Лицензия",
    ] {
        assert!(
            readme.contains(required_section),
            "README lacks public section {required_section}"
        );
    }

    for required_contract in [
        "WorldEngine::commit",
        "MechanismContract",
        "ResolutionContract",
        "MorphotypeDefinition",
        "CortexProposal",
        "CognitiveDisposition",
        "Human",
        "Neko",
        "Phase 0",
        "Phase 1",
        "AGPL-3.0-only",
    ] {
        assert!(
            readme.contains(required_contract),
            "README does not explain {required_contract}"
        );
    }

    assert!(
        !readme.contains("/home/"),
        "README must not expose machine-specific paths"
    );
}

#[test]
fn repository_exposes_public_community_health_and_ci_files() {
    let root = repository_root();
    for required_path in [
        "LICENSE",
        "CONTRIBUTING.md",
        "CODE_OF_CONDUCT.md",
        ".editorconfig",
        ".github/workflows/ci.yml",
        ".github/dependabot.yml",
        ".github/ISSUE_TEMPLATE/bug_report.yml",
        ".github/ISSUE_TEMPLATE/feature_request.yml",
        ".github/ISSUE_TEMPLATE/config.yml",
        ".github/PULL_REQUEST_TEMPLATE.md",
    ] {
        assert!(
            root.join(required_path).is_file(),
            "public repository lacks {required_path}"
        );
    }

    let license = fs::read_to_string(root.join("LICENSE")).expect("read LICENSE");
    assert!(license.contains("GNU AFFERO GENERAL PUBLIC LICENSE"));
    assert!(license.contains("Version 3, 19 November 2007"));

    let workflow =
        fs::read_to_string(root.join(".github/workflows/ci.yml")).expect("read CI workflow");
    for required_gate in [
        "cargo fmt --all -- --check",
        "cargo clippy --workspace --all-targets -- -D warnings",
        "cargo test --workspace --all-targets",
        "ctest --test-dir build/brain --output-on-failure",
    ] {
        assert!(
            workflow.contains(required_gate),
            "CI does not run {required_gate}"
        );
    }
    assert!(workflow.contains("permissions:\n  contents: read"));
}

#[test]
fn every_public_markdown_document_is_reachable_from_readme() {
    let root = repository_root();
    let mut public_documents = BTreeSet::new();
    collect_markdown(&root, &root, &mut public_documents);

    let mut reachable = BTreeSet::from([PathBuf::from("README.md")]);
    let mut queue = VecDeque::from([PathBuf::from("README.md")]);
    while let Some(relative_path) = queue.pop_front() {
        let document = fs::read_to_string(root.join(&relative_path))
            .unwrap_or_else(|error| panic!("read {}: {error}", relative_path.display()));
        let parent = relative_path.parent().unwrap_or(Path::new(""));
        for target in markdown_link_targets(&document) {
            let Some(linked_document) = resolve_markdown_target(&root, parent, target) else {
                continue;
            };
            if reachable.insert(linked_document.clone()) {
                queue.push_back(linked_document);
            }
        }
    }

    let unreachable = public_documents.difference(&reachable).collect::<Vec<_>>();
    assert!(
        unreachable.is_empty(),
        "public Markdown is not reachable from README: {unreachable:#?}"
    );
}

#[test]
fn public_documents_consistently_describe_the_current_architecture() {
    let root = repository_root();
    let mut public_documents = BTreeSet::new();
    collect_markdown(&root, &root, &mut public_documents);
    for relative_path in &public_documents {
        let document = fs::read_to_string(root.join(relative_path)).expect("read public document");
        assert!(
            !document.contains("/home/"),
            "{} exposes a machine-specific path",
            relative_path.display()
        );
        assert!(
            !document.contains("Статус: зафиксированная архитектурная база V1"),
            "{} retains superseded normative status",
            relative_path.display()
        );
    }

    let memory = fs::read_to_string(root.join("MEMORY.md")).expect("read memory design");
    for required in [
        "Статус: нормативный Phase 0 design",
        "Consciousness",
        "CortexProposal",
        "CognitiveDisposition",
        "WorldEngine::commit",
        "нескольких сознаний",
    ] {
        assert!(memory.contains(required), "MEMORY.md lacks {required}");
    }

    let security = fs::read_to_string(root.join("SECURITY.md")).expect("read security policy");
    for required in [
        "## Сообщение об уязвимости",
        "WorldEngine::commit",
        "CortexProposal",
        "CognitiveDisposition",
        "ResolutionChanged",
        "SafeStop",
        "content digest",
    ] {
        assert!(security.contains(required), "SECURITY.md lacks {required}");
    }

    let historical = fs::read_to_string(root.join("STAGE_5.md")).expect("read history");
    assert!(
        historical
            .lines()
            .take(10)
            .any(|line| line == "Статус: superseded"),
        "historical Stage 5 must have unambiguous superseded status"
    );

    let authority =
        fs::read_to_string(root.join("docs/adr/0002-world-authority.md")).expect("read ADR-0002");
    assert!(authority.contains("WorldEngine::commit"));
    assert!(authority.contains("единственный mutation path"));

    let transport =
        fs::read_to_string(root.join("docs/adr/0004-world-service-uds.md")).expect("read ADR-0004");
    assert!(transport.contains("legacy runtime adapter"));
    assert!(transport.contains("не добавляет mutation path"));
}

#[test]
fn public_architecture_defines_timeline_graph_layers_and_resolution_transitions() {
    let root = repository_root();
    let readme = fs::read_to_string(root.join("README.md")).expect("read README");
    let architecture = fs::read_to_string(root.join("ARCHITECTURE.md")).expect("read architecture");
    let invariants = fs::read_to_string(root.join("INVARIANTS.md")).expect("read invariants");

    for required in [
        "Durable causal timeline",
        "единый causal graph",
        "не последовательный pipeline",
        "mixed resolution",
    ] {
        assert!(readme.contains(required), "README lacks {required}");
    }

    for required in [
        "## 3. Единый causal graph",
        "`WORLD EVENTS` не является simulation layer",
        "Explicit Causally Triggered Resolution Transition",
        "control plane",
    ] {
        assert!(
            architecture.contains(required),
            "ARCHITECTURE.md lacks {required}"
        );
    }

    for required in [
        "deterministic trigger",
        "causal relevance",
        "не является скрытым LOD",
        "активация нового artifact",
    ] {
        assert!(
            invariants.contains(required),
            "INVARIANTS.md lacks {required}"
        );
    }
}

#[test]
fn crates_and_repository_publish_consistent_project_metadata() {
    let root = repository_root();
    assert!(
        root.join("CHANGELOG.md").is_file(),
        "repository lacks changelog"
    );
    assert!(
        root.join(".gitattributes").is_file(),
        "repository lacks text attributes"
    );

    assert_eq!(env!("CARGO_PKG_LICENSE"), "AGPL-3.0-only");
    assert_eq!(
        env!("CARGO_PKG_REPOSITORY"),
        "https://github.com/Khalwaia/MakiseWE"
    );
    assert_eq!(
        env!("CARGO_PKG_HOMEPAGE"),
        "https://github.com/Khalwaia/MakiseWE"
    );
    assert!(!env!("CARGO_PKG_DESCRIPTION").is_empty());
    assert_eq!(env!("CARGO_PKG_RUST_VERSION"), "1.97");

    for manifest in ["world/Cargo.toml", "proto/Cargo.toml"] {
        let manifest = fs::read_to_string(root.join(manifest)).expect("read crate manifest");
        for inherited_field in [
            "description.workspace = true",
            "homepage.workspace = true",
            "repository.workspace = true",
            "readme.workspace = true",
        ] {
            assert!(
                manifest.contains(inherited_field),
                "crate manifest does not inherit {inherited_field}"
            );
        }
    }
}

#[test]
fn architecture_decisions_publish_consistent_status_and_date_metadata() {
    let adr_directory = repository_root().join("docs/adr");
    for entry in fs::read_dir(&adr_directory).expect("read ADR directory") {
        let path = entry.expect("read ADR entry").path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !name
            .chars()
            .next()
            .is_some_and(|value| value.is_ascii_digit())
            || path.extension().and_then(|value| value.to_str()) != Some("md")
        {
            continue;
        }
        let adr = fs::read_to_string(&path).expect("read ADR");
        assert!(adr.starts_with("---\n"), "{name} lacks YAML front matter");
        let (metadata, body) = adr[4..]
            .split_once("\n---\n")
            .unwrap_or_else(|| panic!("{name} has unterminated YAML front matter"));
        assert!(
            metadata.contains("status: "),
            "{name} lacks status metadata"
        );
        assert!(metadata.contains("date: "), "{name} lacks date metadata");
        assert!(body.trim_start().starts_with("# "), "{name} lacks title");
    }
}

fn collect_markdown(root: &Path, directory: &Path, output: &mut BTreeSet<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read documentation directory") {
        let path = entry.expect("read documentation entry").path();
        if path.is_dir() {
            let name = path.file_name().and_then(|value| value.to_str());
            if matches!(
                name,
                Some(".git" | ".agents" | ".github" | "graphify-out" | "target")
            ) {
                continue;
            }
            collect_markdown(root, &path, output);
        } else if path.extension().and_then(|value| value.to_str()) == Some("md") {
            output.insert(
                path.strip_prefix(root)
                    .expect("repository path")
                    .to_path_buf(),
            );
        }
    }
}

fn markdown_link_targets(markdown: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut remaining = markdown;
    while let Some(start) = remaining.find("](") {
        remaining = &remaining[start + 2..];
        let Some(end) = remaining.find(')') else {
            break;
        };
        targets.push(&remaining[..end]);
        remaining = &remaining[end + 1..];
    }
    targets
}

fn resolve_markdown_target(root: &Path, parent: &Path, target: &str) -> Option<PathBuf> {
    if target.is_empty()
        || target.starts_with('#')
        || target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
    {
        return None;
    }
    let target = target
        .trim_matches(['<', '>'])
        .split('#')
        .next()
        .expect("link path");
    let mut resolved = parent.join(target);
    if root.join(&resolved).is_dir() {
        resolved = resolved.join("README.md");
    }
    (resolved.extension().and_then(|value| value.to_str()) == Some("md")
        && root.join(&resolved).is_file())
    .then_some(resolved)
}
