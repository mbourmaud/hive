use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Claude wrapper profile (duplicated for testing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub description: Option<String>,
    pub claude_wrapper: String,
    pub environment: Option<Vec<(String, String)>>,
    pub created: String,
    pub updated: String,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            description: Some("Default Claude profile".to_string()),
            claude_wrapper: "claude".to_string(),
            environment: None,
            created: "2024-01-01T00:00:00Z".to_string(),
            updated: "2024-01-01T00:00:00Z".to_string(),
        }
    }
}

fn setup_test_env(temp_dir: &TempDir) -> PathBuf {
    let profiles_dir = temp_dir.path().join("profiles");
    fs::create_dir_all(&profiles_dir).unwrap();
    profiles_dir
}

#[test]
fn test_profile_serialization() {
    let profile = Profile {
        name: "test".to_string(),
        description: Some("Test profile".to_string()),
        claude_wrapper: "claude".to_string(),
        environment: Some(vec![
            ("KEY1".to_string(), "value1".to_string()),
            ("KEY2".to_string(), "value2".to_string()),
        ]),
        created: "2024-01-01T00:00:00Z".to_string(),
        updated: "2024-01-01T00:00:00Z".to_string(),
    };

    let json = serde_json::to_string(&profile).unwrap();
    let parsed: Profile = serde_json::from_str(&json).unwrap();

    assert_eq!(profile.name, parsed.name);
    assert_eq!(profile.description, parsed.description);
    assert_eq!(profile.claude_wrapper, parsed.claude_wrapper);
    assert_eq!(profile.environment, parsed.environment);
}

#[test]
fn test_default_profile() {
    let profile = Profile::default();

    assert_eq!(profile.name, "default");
    assert_eq!(profile.claude_wrapper, "claude");
    assert!(profile.description.is_some());
}

#[test]
fn test_create_profile_file() {
    let temp_dir = TempDir::new().unwrap();
    let profiles_dir = setup_test_env(&temp_dir);

    let profile = Profile {
        name: "test-profile".to_string(),
        description: Some("Test description".to_string()),
        claude_wrapper: "/usr/local/bin/claude".to_string(),
        environment: None,
        created: "2024-01-01T00:00:00Z".to_string(),
        updated: "2024-01-01T00:00:00Z".to_string(),
    };

    let profile_path = profiles_dir.join("test-profile.json");
    let json = serde_json::to_string_pretty(&profile).unwrap();
    fs::write(&profile_path, json).unwrap();

    assert!(profile_path.exists());

    // Read and verify
    let contents = fs::read_to_string(&profile_path).unwrap();
    let loaded: Profile = serde_json::from_str(&contents).unwrap();

    assert_eq!(loaded.name, "test-profile");
    assert_eq!(loaded.description, Some("Test description".to_string()));
}

#[test]
fn test_list_profiles() {
    let temp_dir = TempDir::new().unwrap();
    let profiles_dir = setup_test_env(&temp_dir);

    // Create multiple profiles
    for i in 1..=3 {
        let profile = Profile {
            name: format!("profile-{}", i),
            description: Some(format!("Description {}", i)),
            claude_wrapper: "claude".to_string(),
            environment: None,
            created: "2024-01-01T00:00:00Z".to_string(),
            updated: "2024-01-01T00:00:00Z".to_string(),
        };

        let profile_path = profiles_dir.join(format!("profile-{}.json", i));
        let json = serde_json::to_string_pretty(&profile).unwrap();
        fs::write(&profile_path, json).unwrap();
    }

    // Count profiles
    let count = fs::read_dir(&profiles_dir)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .ok()
                .map(|e| {
                    e.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext == "json")
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        })
        .count();

    assert_eq!(count, 3);
}

#[test]
fn test_active_profile() {
    let temp_dir = TempDir::new().unwrap();
    let profiles_dir = setup_test_env(&temp_dir);

    // Create a profile
    let profile = Profile {
        name: "active-profile".to_string(),
        description: None,
        claude_wrapper: "claude".to_string(),
        environment: None,
        created: "2024-01-01T00:00:00Z".to_string(),
        updated: "2024-01-01T00:00:00Z".to_string(),
    };

    let profile_path = profiles_dir.join("active-profile.json");
    let json = serde_json::to_string_pretty(&profile).unwrap();
    fs::write(&profile_path, json).unwrap();

    // Set as active
    let active_path = profiles_dir.join(".active");
    fs::write(&active_path, "active-profile").unwrap();

    // Verify
    assert!(active_path.exists());
    let active_name = fs::read_to_string(&active_path).unwrap();
    assert_eq!(active_name, "active-profile");
}

#[test]
fn test_delete_profile() {
    let temp_dir = TempDir::new().unwrap();
    let profiles_dir = setup_test_env(&temp_dir);

    // Create a profile
    let profile = Profile {
        name: "to-delete".to_string(),
        description: None,
        claude_wrapper: "claude".to_string(),
        environment: None,
        created: "2024-01-01T00:00:00Z".to_string(),
        updated: "2024-01-01T00:00:00Z".to_string(),
    };

    let profile_path = profiles_dir.join("to-delete.json");
    let json = serde_json::to_string_pretty(&profile).unwrap();
    fs::write(&profile_path, json).unwrap();

    assert!(profile_path.exists());

    // Delete
    fs::remove_file(&profile_path).unwrap();
    assert!(!profile_path.exists());
}

#[test]
fn test_profile_with_environment() {
    let profile = Profile {
        name: "env-profile".to_string(),
        description: None,
        claude_wrapper: "claude".to_string(),
        environment: Some(vec![
            ("API_KEY".to_string(), "secret123".to_string()),
            ("DEBUG".to_string(), "true".to_string()),
        ]),
        created: "2024-01-01T00:00:00Z".to_string(),
        updated: "2024-01-01T00:00:00Z".to_string(),
    };

    assert!(profile.environment.is_some());
    let env = profile.environment.unwrap();
    assert_eq!(env.len(), 2);
    assert_eq!(env[0].0, "API_KEY");
    assert_eq!(env[1].1, "true");
}
