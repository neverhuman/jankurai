use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct DetectedItem {
    pub name: String,
    pub evidence: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiSurface {
    pub framework: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContractEvidence {
    pub kind: String,
    pub path: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StackInventory {
    pub languages: Vec<DetectedItem>,
    pub frameworks: Vec<DetectedItem>,
    pub db_clients: Vec<DetectedItem>,
    pub test_frameworks: Vec<DetectedItem>,
    pub package_managers: Vec<DetectedItem>,
    pub ci_systems: Vec<DetectedItem>,
    pub api_surfaces: Vec<ApiSurface>,
    pub contract_evidence: Vec<ContractEvidence>,
}

pub fn detect_stack(repo: &Path) -> StackInventory {
    let mut inv = StackInventory {
        languages: vec![],
        frameworks: vec![],
        db_clients: vec![],
        test_frameworks: vec![],
        package_managers: vec![],
        ci_systems: vec![],
        api_surfaces: vec![],
        contract_evidence: vec![],
    };

    if repo.join("Cargo.toml").exists() {
        inv.languages.push(di("rust", "Cargo.toml", "high"));
        inv.package_managers.push(di("cargo", "Cargo.toml", "high"));
        inv.test_frameworks
            .push(di("cargo-test", "Cargo.toml", "high"));
        if let Ok(text) = fs::read_to_string(repo.join("Cargo.toml")) {
            let lower = text.to_ascii_lowercase();
            for fw in ["actix", "axum", "rocket", "warp"] {
                if lower.contains(fw) {
                    inv.frameworks.push(di(fw, "Cargo.toml", "medium"));
                    inv.api_surfaces.push(ApiSurface {
                        framework: fw.to_string(),
                        evidence: "Cargo.toml dependency".to_string(),
                    });
                }
            }
            for db in ["sqlx", "diesel", "sea-orm", "tokio-postgres"] {
                if lower.contains(db) {
                    inv.db_clients.push(di(db, "Cargo.toml", "medium"));
                }
            }
        }
    }

    if repo.join("package.json").exists() {
        inv.languages
            .push(di("typescript", "package.json", "medium"));
        inv.package_managers.push(di("npm", "package.json", "high"));
        if let Ok(text) = fs::read_to_string(repo.join("package.json")) {
            let lower = text.to_ascii_lowercase();
            for fw in [
                "express", "fastify", "next", "nuxt", "react", "vue", "angular", "svelte",
            ] {
                if lower.contains(fw) {
                    inv.frameworks.push(di(fw, "package.json", "medium"));
                    if matches!(fw, "express" | "fastify" | "next" | "nuxt") {
                        inv.api_surfaces.push(ApiSurface {
                            framework: fw.to_string(),
                            evidence: "package.json dependency".to_string(),
                        });
                    }
                }
            }
            for tf in ["jest", "vitest", "mocha", "playwright", "cypress"] {
                if lower.contains(tf) {
                    inv.test_frameworks.push(di(tf, "package.json", "medium"));
                }
            }
            for db in ["prisma", "knex", "typeorm", "sequelize", "drizzle"] {
                if lower.contains(db) {
                    inv.db_clients.push(di(db, "package.json", "medium"));
                }
            }
        }
    }

    if repo.join("python").exists() {
        inv.languages.push(di("python", "python/", "medium"));
    }

    if repo.join("pom.xml").exists() || repo.join("build.gradle").exists() {
        let evidence = if repo.join("pom.xml").exists() {
            "pom.xml"
        } else {
            "build.gradle"
        };
        inv.languages.push(di("java", evidence, "high"));
        if repo.join("pom.xml").exists() {
            inv.package_managers.push(di("maven", "pom.xml", "high"));
        }
        if repo.join("build.gradle").exists() {
            inv.package_managers
                .push(di("gradle", "build.gradle", "high"));
        }
        inv.test_frameworks.push(di("junit", evidence, "medium"));
        if let Ok(text) = fs::read_to_string(repo.join("pom.xml")) {
            if text.to_ascii_lowercase().contains("spring") {
                inv.frameworks.push(di("spring", "pom.xml", "medium"));
                inv.api_surfaces.push(ApiSurface {
                    framework: "spring".to_string(),
                    evidence: "pom.xml dependency".to_string(),
                });
            }
        }
    }

    if repo.join("Gemfile").exists() {
        inv.languages.push(di("ruby", "Gemfile", "high"));
        inv.package_managers.push(di("bundler", "Gemfile", "high"));
        inv.test_frameworks.push(di("rspec", "Gemfile", "medium"));
        inv.frameworks.push(di("rails", "Gemfile", "medium"));
        inv.api_surfaces.push(ApiSurface {
            framework: "rails".to_string(),
            evidence: "Gemfile".to_string(),
        });
    }

    if repo.join("composer.json").exists() {
        inv.languages.push(di("php", "composer.json", "high"));
        inv.package_managers
            .push(di("composer", "composer.json", "high"));
        if let Ok(text) = fs::read_to_string(repo.join("composer.json")) {
            if text.to_ascii_lowercase().contains("laravel") {
                inv.frameworks
                    .push(di("laravel", "composer.json", "medium"));
                inv.api_surfaces.push(ApiSurface {
                    framework: "laravel".to_string(),
                    evidence: "composer.json dependency".to_string(),
                });
            }
        }
    }

    if repo.join("go.mod").exists() {
        inv.languages.push(di("go", "go.mod", "high"));
        inv.package_managers
            .push(di("go-modules", "go.mod", "high"));
        inv.test_frameworks.push(di("go-test", "go.mod", "high"));
    }

    if repo.join(".github/workflows").exists() {
        inv.ci_systems
            .push(di("github-actions", ".github/workflows/", "high"));
    }
    if repo.join(".gitlab-ci.yml").exists() {
        inv.ci_systems
            .push(di("gitlab-ci", ".gitlab-ci.yml", "high"));
    }
    if repo.join(".circleci").exists() {
        inv.ci_systems.push(di("circleci", ".circleci/", "high"));
    }

    for (kind, glob_pattern) in [
        ("openapi", "openapi.yaml"),
        ("openapi", "openapi.json"),
        ("openapi", "swagger.json"),
        ("proto", "*.proto"),
        ("graphql", "schema.graphql"),
    ] {
        if repo.join(glob_pattern).exists() {
            inv.contract_evidence.push(ContractEvidence {
                kind: kind.to_string(),
                path: glob_pattern.to_string(),
                status: "detected".to_string(),
            });
        }
    }
    if repo.join("contracts").exists() {
        inv.contract_evidence.push(ContractEvidence {
            kind: "directory".to_string(),
            path: "contracts/".to_string(),
            status: "detected".to_string(),
        });
    }
    if repo.join("schemas").exists() {
        inv.contract_evidence.push(ContractEvidence {
            kind: "directory".to_string(),
            path: "schemas/".to_string(),
            status: "detected".to_string(),
        });
    }

    inv
}

fn di(name: &str, evidence: &str, confidence: &str) -> DetectedItem {
    DetectedItem {
        name: name.to_string(),
        evidence: evidence.to_string(),
        confidence: confidence.to_string(),
    }
}

pub fn has_lockfile(repo: &Path) -> bool {
    repo.join("Cargo.lock").exists()
        || repo.join("package-lock.json").exists()
        || repo.join("pnpm-lock.yaml").exists()
        || repo.join("yarn.lock").exists()
        || repo.join("Gemfile.lock").exists()
        || repo.join("composer.lock").exists()
        || repo.join("go.sum").exists()
}
