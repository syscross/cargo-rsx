use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct RsxError {
    pub message: String,
}

/// 라인 단위로 `//` 이후를 지운다. 길이는 유지해서 줄 번호가 안 어긋나게 한다.
fn strip_line_comments(src: &str) -> String {
    src.lines()
        .map(|line| {
            if let Some(idx) = line.find("//") {
                let mut s = line[..idx].to_string();
                s.push_str(&" ".repeat(line.len() - idx));
                s
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn check(src: &str) -> Result<(), Vec<RsxError>> {
    let clean = strip_line_comments(src);
    let structs = collect_structs(&clean);
    let errors = check_uninitialized_use(&clean, &structs);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

type StructMap = HashMap<String, Vec<String>>;

fn collect_structs(src: &str) -> StructMap {
    let mut map = HashMap::new();
    let struct_re = Regex::new(r"struct\s+(\w+)\s*\{").unwrap();

    for cap in struct_re.captures_iter(src) {
        let name = cap[1].to_string();
        let open = cap.get(0).unwrap().end();
        if let Some(close) = find_matching_brace(src, open) {
            let body = &src[open..close];
            let field_re = Regex::new(r"(?m)^\s*(\w+)\s*:\s*[\w:<>]+\s*,").unwrap();
            let fields: Vec<String> = field_re
                .captures_iter(body)
                .map(|c| c[1].to_string())
                .collect();
            map.insert(name, fields);
        }
    }

    map
}

/// `let x: Type;` 이후, Type의 모든 필드가 `x.field = ...;`로 채워지기 전에
/// `x.method()`가 먼저 나오면 거부.
fn check_uninitialized_use(src: &str, structs: &StructMap) -> Vec<RsxError> {
    let mut errors = Vec::new();

    let let_re = Regex::new(r"\blet\s+(\w+)\s*:\s*(\w+)\s*;").unwrap();

    for cap in let_re.captures_iter(src) {
        let var = &cap[1];
        let type_name = &cap[2];
        let decl_end = cap.get(0).unwrap().end();

        let Some(fields) = structs.get(type_name) else { continue };
        if fields.is_empty() {
            continue;
        }

        let rest = &src[decl_end..];

        let assign_re = Regex::new(&format!(r"\b{var}\.(\w+)\s*=")).unwrap();
        let call_re = Regex::new(&format!(r"\b{var}\.(\w+)\s*\(")).unwrap();

        let mut events: Vec<(usize, bool, String)> = Vec::new();
        for c in assign_re.captures_iter(rest) {
            events.push((c.get(0).unwrap().start(), true, c[1].to_string()));
        }
        for c in call_re.captures_iter(rest) {
            events.push((c.get(0).unwrap().start(), false, c[1].to_string()));
        }
        events.sort_by_key(|e| e.0);

        let mut assigned: HashSet<String> = HashSet::new();
        for (pos, is_assign, name) in events {
            if is_assign {
                assigned.insert(name);
            } else {
                let missing: Vec<&String> =
                    fields.iter().filter(|f| !assigned.contains(*f)).collect();
                if !missing.is_empty() {
                    let line = line_of(src, decl_end + pos);
                    let missing_str = missing
                        .iter()
                        .map(|s| format!("`{s}`"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    errors.push(RsxError {
                        message: format!(
                            "error: use of possibly-uninitialized value `{var}` at line {line} — missing field(s): {missing_str}"
                        ),
                    });
                }
                break;
            }
        }
    }

    errors
}

pub fn transpile(src: &str) -> String {
    let structs = collect_structs(src);
    lower_partial_init_to_literal(src, &structs)
}

/// `let a: Type;` 뒤에 `a.field = expr;`이 필드 전부에 대해 나오면,
/// 그 블록을 `let a = Type { field: expr, ... };` 하나로 접는다.
fn lower_partial_init_to_literal(src: &str, structs: &StructMap) -> String {
    let let_re = Regex::new(r"(?m)^(\s*)let\s+(\w+)\s*:\s*(\w+)\s*;\s*\n").unwrap();

    let mut out = src.to_string();

    let snapshot = out.clone();
    let matches: Vec<_> = let_re.captures_iter(&snapshot).collect();

    for cap in matches.iter().rev() {
        let indent = &cap[1];
        let var = cap[2].to_string();
        let type_name = cap[3].to_string();
        let decl_start = cap.get(0).unwrap().start();
        let decl_end = cap.get(0).unwrap().end();

        let Some(fields) = structs.get(&type_name) else { continue };
        if fields.is_empty() {
            continue;
        }

        let assign_re = Regex::new(&format!(
            r"(?m)^\s*{var}\.(\w+)\s*=\s*([^;]+);\s*\n"
        ))
        .unwrap();

        let rest = out[decl_end..].to_string();
        let assigns: Vec<(String, String, std::ops::Range<usize>)> = assign_re
            .captures_iter(&rest)
            .map(|c| {
                let m = c.get(0).unwrap();
                (c[1].to_string(), c[2].trim().to_string(), m.start()..m.end())
            })
            .collect();

        let assigned_names: HashSet<&str> =
            assigns.iter().map(|(n, _, _)| n.as_str()).collect();
        let all_fields_covered = fields.iter().all(|f| assigned_names.contains(f.as_str()));
        if !all_fields_covered {
            continue;
        }

        let last_end = assigns.iter().map(|(_, _, r)| r.end).max().unwrap();

        let mut by_name: HashMap<&str, &str> = HashMap::new();
        for (name, val, _) in &assigns {
            by_name.insert(name.as_str(), val.as_str());
        }
        let literal_fields: String = fields
            .iter()
            .map(|f| format!("{f}: {}", by_name[f.as_str()]))
            .collect::<Vec<_>>()
            .join(", ");

        let literal = format!("{indent}let {var} = {type_name} {{ {literal_fields} }};\n");

        let abs_last = decl_end + last_end;

        out.replace_range(decl_start..abs_last, &literal);
    }

    out
}

fn find_matching_brace(src: &str, open_pos: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let mut depth = 1;
    let mut i = open_pos;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn line_of(src: &str, byte_pos: usize) -> usize {
    src[..byte_pos].matches('\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_partial_init_into_literal() {
        let src = r#"struct Hello {
    name: String,
}

impl Hello {
    fn hello(&self) {
        println!("Hello {}", self.name);
    }
}

fn main() {
    let a: Hello;

    a.name = "RSX".to_string();

    a.hello();
}
"#;
        assert!(check(src).is_ok());
        let out = transpile(src);
        assert!(out.contains("let a = Hello { name: \"RSX\".to_string() };"));
    }

    #[test]
    fn rejects_missing_field() {
        let src = r#"struct Hello {
    name: String,
    desc: String,
}

fn main() {
    let a: Hello;
    a.name = "RSX".to_string();

    a.hello();
}
"#;
        let result = check(src);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("desc"));
    }
}
