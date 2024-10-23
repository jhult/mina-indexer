use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

fn find_rust_files(directory: &Path) -> Vec<PathBuf> {
    WalkDir::new(directory)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.extension().map_or(false, |ext| ext == "rs")
                && path.to_str().unwrap().starts_with("../src/protocol/")
                && !path.to_str().unwrap().contains("/target/")
                && !path.to_str().unwrap().contains("/tests/")
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

fn extract_public_items(file_path: &Path) -> (Vec<String>, Vec<String>, Vec<String>) {
    let content = fs::read_to_string(file_path).unwrap();
    let type_re = Regex::new(r"(?m)^\s*pub\s+type\s+(\w+)(?:<.*>)?").unwrap();
    let struct_re = Regex::new(r"(?m)^\s*pub\s+struct\s+(\w+)(?:<.*>)?").unwrap();
    let enum_re = Regex::new(r"(?m)^\s*pub\s+enum\s+(\w+)(?:<.*>)?").unwrap();

    let types: Vec<String> = type_re
        .captures_iter(&content)
        .map(|cap| cap[1].to_string())
        .collect();

    let structs: Vec<String> = struct_re
        .captures_iter(&content)
        .map(|cap| cap[1].to_string())
        .collect();

    let enums: Vec<String> = enum_re
        .captures_iter(&content)
        .map(|cap| cap[1].to_string())
        .collect();

    println!(
        "Found {} public types, {} public structs, and {} public enums in {:?}",
        types.len(),
        structs.len(),
        enums.len(),
        file_path
    );

    (types, structs, enums)
}

fn find_item_usages(item_name: &str, directory: &Path, exclude_file: &Path) -> bool {
    for entry in WalkDir::new(directory).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "rs")
            && path != exclude_file
            && !path.to_str().unwrap().contains("/target/")
            && !path.to_str().unwrap().contains("/tests/")
        {
            let content = fs::read_to_string(path).unwrap();
            if content.contains(item_name)
                || content.contains(&format!("type {} =", item_name))
                || content.contains(&format!(": {}", item_name))
                || content.contains(&format!("-> {}", item_name))
            {
                return true;
            }
        }
    }
    false
}

fn find_nested_usages(item_name: &str, directory: &Path) -> bool {
    for entry in WalkDir::new(directory).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "rs")
            && !path.to_str().unwrap().contains("/target/")
            && !path.to_str().unwrap().contains("/tests/")
        {
            let content = fs::read_to_string(path).unwrap();
            if content.contains(&format!("pub struct")) || content.contains(&format!("pub enum")) {
                if content.contains(item_name) {
                    return true;
                }
            }
        }
    }
    false
}

fn analyze_items(
    workspace_dir: &Path,
) -> (
    HashMap<PathBuf, HashSet<String>>,
    HashMap<PathBuf, HashSet<String>>,
    HashMap<PathBuf, HashSet<String>>,
) {
    let mut types_to_keep_public = HashMap::new();
    let mut structs_to_keep_public = HashMap::new();
    let mut enums_to_keep_public = HashMap::new();

    let protocol_dir = workspace_dir.join("src/protocol");
    for file_path in find_rust_files(&protocol_dir) {
        let (types, structs, enums) = extract_public_items(&file_path);

        for type_name in types {
            if find_item_usages(&type_name, workspace_dir, &file_path)
                || find_nested_usages(&type_name, workspace_dir)
            {
                types_to_keep_public
                    .entry(file_path.clone())
                    .or_insert_with(HashSet::new)
                    .insert(type_name);
            }
        }

        for struct_name in structs {
            if find_item_usages(&struct_name, workspace_dir, &file_path)
                || find_nested_usages(&struct_name, workspace_dir)
            {
                structs_to_keep_public
                    .entry(file_path.clone())
                    .or_insert_with(HashSet::new)
                    .insert(struct_name);
            }
        }

        for enum_name in enums {
            if find_item_usages(&enum_name, workspace_dir, &file_path)
                || find_nested_usages(&enum_name, workspace_dir)
            {
                enums_to_keep_public
                    .entry(file_path.clone())
                    .or_insert_with(HashSet::new)
                    .insert(enum_name);
            }
        }
    }

    (
        types_to_keep_public,
        structs_to_keep_public,
        enums_to_keep_public,
    )
}

fn find_item_positions(content: &str, item_type: &str) -> Vec<(String, usize, usize)> {
    let re = match item_type {
        "type" => Regex::new(r"^\s*pub\s+type\s+(\w+)(?:<.*>)?").unwrap(),
        "struct" => Regex::new(r"^\s*pub\s+struct\s+(\w+)(?:<.*>)?").unwrap(),
        "enum" => Regex::new(r"^\s*pub\s+enum\s+(\w+)(?:<.*>)?").unwrap(),
        _ => panic!("Invalid item type"),
    };

    content
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            re.captures(line).map(|cap| {
                let name = cap[1].to_string();
                (name, i, i + 1)
            })
        })
        .collect()
}

fn modify_single_item(content: &[String], start: usize, item_type: &str) -> (Vec<String>, bool) {
    let mut modified = content.to_vec();
    let original_line = &content[start];

    let new_line = match item_type {
        "type" => original_line.replace("pub type", "pub(self) type"),
        "struct" => original_line.replace("pub struct", "pub(self) struct"),
        "enum" => original_line.replace("pub enum", "pub(self) enum"),
        _ => panic!("Invalid item type"),
    };

    if original_line != &new_line {
        modified[start] = new_line;
        (modified, true)
    } else {
        (modified, false)
    }
}

fn make_item_private(content: &[String], start: usize, item_type: &str) -> (Vec<String>, bool) {
    let mut modified = content.to_vec();
    let original_line = &content[start];

    let new_line = match item_type {
        "type" => original_line
            .replace("pub type", "type")
            .replace("pub(self) type", "type"),
        "struct" => original_line
            .replace("pub struct", "struct")
            .replace("pub(self) struct", "struct"),
        "enum" => original_line
            .replace("pub enum", "enum")
            .replace("pub(self) enum", "enum"),
        _ => panic!("Invalid item type"),
    };

    if original_line != &new_line {
        modified[start] = new_line;
        (modified, true)
    } else {
        (modified, false)
    }
}

fn modify_enum_variants(content: &[String], enum_name: &str) -> (Vec<String>, bool) {
    let mut modified = content.to_vec();
    let mut changes_made = false;
    let enum_re = Regex::new(&format!(r"^\s*(?:pub|pub\(self\))?\s*enum\s+{}", enum_name)).unwrap();
    let variant_re = Regex::new(r"^\s*pub\s+(\w+)").unwrap();

    let mut in_enum = false;
    let mut brace_count = 0;
    let mut changes = Vec::new();

    for (i, line) in modified.iter().enumerate() {
        if enum_re.is_match(line) {
            in_enum = true;
            brace_count = 1;
            continue;
        }

        if in_enum {
            if line.contains('{') {
                brace_count += 1;
            }
            if line.contains('}') {
                brace_count -= 1;
                if brace_count == 0 {
                    break;
                }
            }

            if let Some(caps) = variant_re.captures(line) {
                let new_line = line.replace("pub ", "");
                changes.push((i, new_line));
                changes_made = true;
                println!(
                    "Made enum variant {} private in enum {}",
                    &caps[1], enum_name
                );
            }
        }
    }

    // Apply the changes after the iteration
    for (i, new_line) in changes {
        modified[i] = new_line;
    }

    (modified, changes_made)
}

fn run_cargo_check(workspace_root: &Path) -> (bool, String) {
    let output = Command::new("cargo")
        .current_dir(workspace_root)
        .arg("check")
        .output()
        .expect("Failed to execute cargo check");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

fn process_items(workspace_dir: &Path, item_type: &str) -> bool {
    let (types_to_keep_public, structs_to_keep_public, enums_to_keep_public) =
        analyze_items(workspace_dir);
    let items_to_keep_public = match item_type {
        "type" => &types_to_keep_public,
        "struct" => &structs_to_keep_public,
        "enum" => &enums_to_keep_public,
        _ => panic!("Invalid item type"),
    };
    let mut changes_made = false;

    for (file_path, public_items) in items_to_keep_public.iter() {
        let original_content = fs::read_to_string(file_path).unwrap();
        let content_lines: Vec<String> = original_content.lines().map(String::from).collect();
        let item_positions = find_item_positions(&original_content, item_type);
        let mut modified_lines = content_lines.clone();

        for (item_name, start, _) in item_positions {
            if !public_items.contains(&item_name) {
                let (new_lines, was_modified) =
                    modify_single_item(&modified_lines, start, item_type);

                if was_modified {
                    modified_lines = new_lines;

                    if item_type == "enum" {
                        let (enum_modified_lines, enum_changes) =
                            modify_enum_variants(&modified_lines, &item_name);
                        if enum_changes {
                            modified_lines = enum_modified_lines;
                        }
                    }

                    let modified_content = modified_lines.join("\n");
                    fs::write(file_path, &modified_content).unwrap();

                    let (check_passed, error_output) = run_cargo_check(workspace_dir);
                    if check_passed {
                        println!(
                            "Successfully made {} {} in {:?} pub(self).",
                            item_type, item_name, file_path
                        );
                        changes_made = true;
                    } else if error_output.contains("is more private than")
                        || error_output.contains("is reachable at visibility `pub`")
                    {
                        // If the error is due to a nested visibility issue, revert the change
                        println!("Cannot modify visibility of {} {} in {:?} due to nested visibility constraints: {}",
                            item_type, item_name, file_path, error_output);
                        fs::write(file_path, &original_content).unwrap(); // Revert the change
                        modified_lines = content_lines.clone();
                    } else if error_output.contains("is private") {
                        // If the error is due to a private item, try making the current item completely private
                        let (private_lines, private_modified) =
                            make_item_private(&modified_lines, start, item_type);
                        if private_modified {
                            fs::write(file_path, &private_lines.join("\n")).unwrap();
                            let (private_check_passed, private_error_output) =
                                run_cargo_check(workspace_dir);
                            if private_check_passed {
                                println!(
                                    "Successfully made {} {} in {:?} private.",
                                    item_type, item_name, file_path
                                );
                                modified_lines = private_lines;
                                changes_made = true;
                            } else {
                                println!("Cannot modify visibility of {} {} in {:?} due to usage constraints: {}",
                                    item_type, item_name, file_path, private_error_output);
                                fs::write(file_path, &original_content).unwrap(); // Revert the change
                                modified_lines = content_lines.clone();
                            }
                        }
                    } else {
                        println!("Cannot modify visibility of {} {} in {:?} due to usage constraints: {}",
                            item_type, item_name, file_path, error_output);
                        fs::write(file_path, &original_content).unwrap(); // Revert the change
                        modified_lines = content_lines.clone();
                    }
                } else {
                    println!(
                        "{} {} in {:?} is already not fully public.",
                        item_type, item_name, file_path
                    );
                }
            } else {
                println!(
                    "Keeping {} {} in {:?} public as it's used elsewhere.",
                    item_type, item_name, file_path
                );
            }
        }
    }

    changes_made
}

fn find_public_struct_fields(content: &str) -> Vec<(String, String)> {
    let struct_re = Regex::new(r"pub\s+struct\s+(\w+)\s*\{([^}]+)\}").unwrap();
    let field_re = Regex::new(r"pub\s+(\w+)\s*:").unwrap();
    let mut fields = Vec::new();

    for struct_cap in struct_re.captures_iter(content) {
        let struct_name = struct_cap[1].to_string();
        let struct_body = &struct_cap[2];
        for line in struct_body.lines() {
            for field_cap in field_re.captures_iter(line) {
                let field_name = field_cap[1].to_string();
                fields.push((struct_name.clone(), field_name));
            }
        }
    }

    fields
}

fn find_field_usages(
    struct_name: &str,
    field_name: &str,
    directory: &Path,
    exclude_file: &Path,
) -> bool {
    for entry in WalkDir::new(directory).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "rs")
            && path != exclude_file
            && !path.to_str().unwrap().contains("/target/")
            && !path.to_str().unwrap().contains("/tests/")
        {
            let content = fs::read_to_string(path).unwrap();
            if content.contains(&format!("{}.{}", struct_name, field_name))
                || content.contains(&format!("{} . {}", struct_name, field_name))
                || content.contains(&format!("{}::{}", struct_name, field_name))
                || content.contains(&format!("{} :: {}", struct_name, field_name))
                || content.contains(&format!("field: {}", field_name))
                || content.contains(&format!("field : {}", field_name))
                || content.contains(&format!("field:{}", field_name))
                || content.contains(&format!("field :{}", field_name))
            {
                return true;
            }
        }
    }
    false
}

fn process_struct_fields(workspace_dir: &Path) -> bool {
    let mut changes_made = false;
    let protocol_dir = workspace_dir.join("src/protocol");

    for file_path in find_rust_files(&protocol_dir) {
        let original_content = fs::read_to_string(&file_path).unwrap();
        let mut modified_content = original_content.clone();
        let fields = find_public_struct_fields(&original_content);

        for (struct_name, field_name) in fields {
            // Check if the struct is public and in a public module
            if is_public_api(&original_content, &struct_name) {
                println!(
                    "Skipping field {} in struct {} as it's part of the public API",
                    field_name, struct_name
                );
                continue;
            }

            if !find_field_usages(&struct_name, &field_name, workspace_dir, &file_path) {
                let field_re = Regex::new(&format!(r"(?m)^\s*pub\s+{}:", field_name)).unwrap();
                if let Some(mat) = field_re.find(&modified_content) {
                    let start = mat.start();
                    let end = modified_content[start..]
                        .find('\n')
                        .map_or(modified_content.len(), |i| start + i);
                    let line = &modified_content[start..end];
                    let new_line = line.replace("pub ", "");
                    modified_content.replace_range(start..end, &new_line);

                    fs::write(&file_path, &modified_content).unwrap();
                    let (check_passed, error_output) = run_cargo_check(workspace_dir);
                    if check_passed {
                        println!(
                            "Successfully made field {} in struct {} private in {:?}",
                            field_name, struct_name, file_path
                        );
                        changes_made = true;
                    } else {
                        println!(
                            "Cannot make field {} in struct {} private in {:?} due to: {}",
                            field_name, struct_name, file_path, error_output
                        );
                        modified_content = original_content.clone(); // Revert changes
                    }
                }
            }
        }

        if modified_content != original_content {
            fs::write(&file_path, &modified_content).unwrap();
        }
    }

    changes_made
}

fn is_public_api(content: &str, struct_name: &str) -> bool {
    let struct_re = Regex::new(&format!(r"pub\s+struct\s+{}", struct_name)).unwrap();
    struct_re.is_match(content)
}

fn make_pub_crate(workspace_dir: &Path) -> bool {
    let mut changes_made = false;
    let protocol_dir = workspace_dir.join("src/protocol");

    for file_path in find_rust_files(&protocol_dir) {
        let original_content = fs::read_to_string(&file_path).unwrap();
        let mut modified_content = original_content.clone();

        // Replace "pub " with "pub(crate) " for structs, enums, and types
        let re = Regex::new(r"(?m)^(\s*)pub\s+(struct|enum|type)\s+").unwrap();
        modified_content = re
            .replace_all(&modified_content, "${1}pub(crate) $2 ")
            .to_string();

        // Replace "pub " with "pub(crate) " for struct fields
        let re = Regex::new(r"(?m)^(\s*)pub\s+(\w+\s*:)").unwrap();
        modified_content = re
            .replace_all(&modified_content, "${1}pub(crate) $2")
            .to_string();

        if modified_content != original_content {
            fs::write(&file_path, &modified_content).unwrap();
            let (check_passed, error_output) = run_cargo_check(workspace_dir);

            if check_passed {
                println!(
                    "Successfully made public items pub(crate) in {:?}",
                    file_path
                );
                changes_made = true;
            } else {
                println!(
                    "Cannot make public items pub(crate) in {:?} due to: {}",
                    file_path, error_output
                );
                fs::write(&file_path, &original_content).unwrap(); // Revert changes
            }
        }
    }

    changes_made
}

fn main() {
    let workspace_dir = Path::new("..");
    let mut iteration = 1;

    loop {
        println!("Starting iteration {}", iteration);

        // Make all public items pub(crate) if possible
        let changes_made_pub_crate = make_pub_crate(workspace_dir);

        // Process struct fields
        let changes_made_fields = process_struct_fields(workspace_dir);

        // Process types
        let changes_made_types = process_items(workspace_dir, "type");

        // Process structs and enums last
        let changes_made_structs = process_items(workspace_dir, "struct");
        let changes_made_enums = process_items(workspace_dir, "enum");

        if !changes_made_pub_crate
            && !changes_made_types
            && !changes_made_fields
            && !changes_made_structs
            && !changes_made_enums
        {
            println!(
                "No changes made in iteration {}. Script complete.",
                iteration
            );
            break;
        }

        println!("Completed iteration {}. Running again...", iteration);
        iteration += 1;
    }

    println!(
        "Finished processing all types, struct fields, structs, and enums under src/protocol/ after {} iterations.",
        iteration
    );
}
